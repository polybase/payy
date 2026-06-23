// lint-long-file-override allow-max-lines=300
use std::path::Path;

use contextful::ResultContextExt;
use json_store::{FileAccess, InvalidJsonBehavior, JsonStore};
use sha2::{Digest, Sha256};

use crate::apps::{
    Error, Result,
    model::{ActionPlan, ApprovalFeeCap, ApprovalRecord, ApprovalStatus, ApprovalsState},
    store::now,
};

pub struct ApprovalStore {
    store: JsonStore<ApprovalsState>,
}

impl ApprovalStore {
    pub async fn load(root: &Path) -> Result<Self> {
        let store = JsonStore::new_with_invalid_json_behavior_and_access(
            root.join("apps"),
            "approvals.json",
            InvalidJsonBehavior::Error,
            FileAccess::OwnerOnly,
        )
        .await
        .context("load beam app approvals")?;
        Ok(Self { store })
    }

    pub async fn list(&self) -> Vec<ApprovalRecord> {
        self.store.get().await.approvals
    }

    pub async fn create(
        &self,
        plan: ActionPlan,
        fee_caps: Vec<ApprovalFeeCap>,
    ) -> Result<ApprovalRecord> {
        let created_at = now();
        let plan_hash = plan_hash(&plan)?;
        let id = format!("apr_{}", &plan_hash["sha256:".len()..18]);
        let record = ApprovalRecord {
            id,
            status: ApprovalStatus::Pending,
            plan,
            plan_hash,
            fee_caps,
            created_at,
            updated_at: created_at,
        };
        self.store
            .update(|state| {
                state.approvals.retain(|approval| approval.id != record.id);
                state.approvals.push(record.clone());
            })
            .await
            .context("persist beam app approval")?;

        Ok(record)
    }

    pub async fn find(&self, id: &str) -> Result<ApprovalRecord> {
        self.store
            .get()
            .await
            .approvals
            .into_iter()
            .find(|approval| approval.id == id)
            .ok_or_else(|| Error::ApprovalNotFound {
                approval_id: id.to_string(),
            })
    }

    pub async fn approve(&self, id: &str) -> Result<ApprovalRecord> {
        let existing = self.find(id).await?;
        ensure_approval_pending(&existing)?;

        let mut selected = None;
        self.store
            .update(|state| {
                for approval in &mut state.approvals {
                    if approval.id == id {
                        approval.status = ApprovalStatus::Approved;
                        approval.updated_at = now();
                        selected = Some(approval.clone());
                    }
                }
            })
            .await
            .context("persist beam app approval status")?;
        let approval = selected.ok_or_else(|| Error::ApprovalNotFound {
            approval_id: id.to_string(),
        })?;
        Ok(approval)
    }

    pub async fn mark_executed(&self, id: &str) -> Result<ApprovalRecord> {
        let existing = self.find(id).await?;
        ensure_approval_executable(&existing)?;

        let mut selected = None;
        self.store
            .update(|state| {
                for approval in &mut state.approvals {
                    if approval.id == id {
                        approval.status = ApprovalStatus::Executed;
                        approval.updated_at = now();
                        selected = Some(approval.clone());
                    }
                }
            })
            .await
            .context("persist beam app approval execution")?;
        selected.ok_or_else(|| Error::ApprovalNotFound {
            approval_id: id.to_string(),
        })
    }

    pub async fn reject(&self, id: &str) -> Result<ApprovalRecord> {
        let mut selected = None;
        self.store
            .update(|state| {
                for approval in &mut state.approvals {
                    if approval.id == id {
                        approval.status = ApprovalStatus::Rejected;
                        approval.updated_at = now();
                        selected = Some(approval.clone());
                    }
                }
            })
            .await
            .context("persist beam app approval rejection")?;
        selected.ok_or_else(|| Error::ApprovalNotFound {
            approval_id: id.to_string(),
        })
    }
}

pub fn ensure_approval_pending(record: &ApprovalRecord) -> Result<()> {
    ensure_approval_integrity(record)?;
    if record.status != ApprovalStatus::Pending {
        return Err(Error::ApprovalNotPending {
            approval_id: record.id.clone(),
        });
    }

    Ok(())
}

pub fn ensure_approval_executable(record: &ApprovalRecord) -> Result<()> {
    ensure_approval_integrity(record)?;
    if !matches!(
        record.status,
        ApprovalStatus::Pending | ApprovalStatus::Approved
    ) {
        return Err(Error::ApprovalNotPending {
            approval_id: record.id.clone(),
        });
    }

    Ok(())
}

pub fn ensure_approval_integrity(record: &ApprovalRecord) -> Result<()> {
    ensure_approval_active(record)?;
    ensure_approval_plan_hash(record)?;
    ensure_uniswap_swap_bindings(record)
}

pub fn ensure_approval_active(record: &ApprovalRecord) -> Result<()> {
    if record.plan.expires_at < now() {
        return Err(Error::ApprovalExpired {
            approval_id: record.id.clone(),
        });
    }

    Ok(())
}

pub fn ensure_approval_plan_hash(record: &ApprovalRecord) -> Result<()> {
    if plan_hash(&record.plan)? != record.plan_hash {
        return Err(Error::ApprovalPlanChanged {
            approval_id: record.id.clone(),
        });
    }

    Ok(())
}

pub fn plan_hash(plan: &ActionPlan) -> Result<String> {
    let bytes = serde_json::to_vec(plan).context("encode beam app action plan")?;
    Ok(format!("sha256:{}", hex::encode(Sha256::digest(bytes))))
}

fn ensure_uniswap_swap_bindings(record: &ApprovalRecord) -> Result<()> {
    if record.plan.app_id != "uniswap" || !record.plan.command.starts_with("swap ") {
        return Ok(());
    }

    for key in [
        "quote_id",
        "quote_expires_at",
        "route_hash",
        "swap_calldata_hash",
        "router",
        "sell_token",
        "buy_token",
        "amount_in",
        "amount_out",
    ] {
        if binding(record, key).is_none() {
            return Err(Error::InvalidGuestOutput {
                reason: format!("uniswap approval missing binding {key}"),
            });
        }
    }

    let quote_expires_at = binding(record, "quote_expires_at")
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| Error::InvalidGuestOutput {
            reason: "uniswap approval has invalid quote_expires_at binding".to_string(),
        })?;
    if quote_expires_at < now() {
        return Err(Error::ApprovalExpired {
            approval_id: record.id.clone(),
        });
    }

    Ok(())
}

fn binding<'a>(record: &'a ApprovalRecord, key: &str) -> Option<&'a str> {
    record
        .plan
        .bindings
        .iter()
        .find(|binding| binding.key == key)
        .map(|binding| binding.value.as_str())
}
