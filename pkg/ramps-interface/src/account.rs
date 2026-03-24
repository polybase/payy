// lint-long-file-override allow-max-lines=400
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use currency::Country;
#[cfg(feature = "diesel")]
use database::schema::ramps_accounts;
#[cfg(feature = "diesel")]
use diesel::prelude::*;
use kyc::{Kyc, KycStatus, KycUpdateRequired};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use test_spy::spy_mock;
use uuid::Uuid;

use crate::error::Result;
use crate::provider::Provider;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "diesel",
    derive(Queryable, Selectable, Identifiable, Insertable, AsChangeset)
)]
#[cfg_attr(feature = "diesel", diesel(primary_key(id)))]
#[cfg_attr(feature = "diesel", diesel(table_name = ramps_accounts))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct Account {
    pub id: Uuid,
    pub provider: Provider,
    pub wallet_id: Uuid,
    pub kyc_status: KycStatus,
    pub kyc_update_required_fields: Option<KycUpdateRequired>,
    pub kyc_external_id: Option<String>,
    pub kyc_delegated_id: Option<Uuid>,
    pub kyc_non_delegated_status: Option<KycStatus>,
    pub country: Option<Country>,
    pub external_id: Option<String>,
    pub withdraw_evm_address: Option<String>,
    pub deposit_evm_address: Option<String>,
    pub metadata: Option<Value>,
    pub updated_at: DateTime<Utc>,
    pub added_at: DateTime<Utc>,
}

impl Default for Account {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::nil(),
            provider: Provider::Alfred,
            wallet_id: Uuid::nil(),
            kyc_status: KycStatus::NotStarted,
            kyc_update_required_fields: None,
            kyc_external_id: None,
            kyc_delegated_id: None,
            kyc_non_delegated_status: None,
            country: None,
            external_id: None,
            withdraw_evm_address: None,
            deposit_evm_address: None,
            metadata: None,
            updated_at: now,
            added_at: now,
        }
    }
}

impl Account {
    #[must_use]
    pub fn email(&self) -> String {
        format!("{}@payy.link", self.id)
    }

    #[must_use]
    pub fn from_request(
        wallet_id: Uuid,
        kyc_delegated_id: Option<Uuid>,
        request: AccountCreateRequest,
    ) -> Self {
        let date = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            wallet_id,
            country: request.country,
            kyc_status: KycStatus::NotStarted,
            kyc_update_required_fields: None,
            kyc_external_id: None,
            kyc_delegated_id,
            kyc_non_delegated_status: None,
            provider: request.provider,
            withdraw_evm_address: None,
            deposit_evm_address: request.evm_address,
            external_id: None,
            metadata: None,
            updated_at: date,
            added_at: date,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "diesel", derive(Queryable, AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = ramps_accounts))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct AccountUpdate {
    pub external_id: Option<String>,
    pub kyc_status: Option<KycStatus>,
    pub kyc_update_required_fields: Option<KycUpdateRequired>,
    pub kyc_external_id: Option<String>,
    pub deposit_evm_address: Option<String>,
    pub withdraw_evm_address: Option<String>,
    pub metadata: Option<Value>,
    pub kyc_non_delegated_status: Option<KycStatus>,
}

impl AccountUpdate {
    #[must_use]
    pub fn without_metadata(&self) -> Self {
        Self {
            metadata: None,
            ..self.clone()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountCreateRequest {
    pub provider: Provider,
    pub country: Option<Country>,
    pub kyc: Option<Kyc>,
    pub evm_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountUpdateRequest {
    pub kyc: Option<Kyc>,
    pub evm_address: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AccountKyc {
    pub kyc_status: KycStatus,
    pub kyc_update_required: Option<KycUpdateRequired>,
}

impl AccountKyc {
    #[must_use]
    pub fn approved() -> Self {
        Self {
            kyc_status: KycStatus::Approved,
            kyc_update_required: None,
        }
    }

    #[must_use]
    pub fn rejected() -> Self {
        Self {
            kyc_status: KycStatus::Rejected,
            kyc_update_required: None,
        }
    }

    #[must_use]
    pub fn rejected_with_reason(reason: Option<KycUpdateRequired>) -> Self {
        Self {
            kyc_status: KycStatus::Rejected,
            kyc_update_required: reason,
        }
    }

    #[must_use]
    pub fn rejected_country() -> Self {
        Self {
            kyc_status: KycStatus::Rejected,
            kyc_update_required: Some(KycUpdateRequired::UnsupportedCountry),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ListAccountsQuery {
    pub provider: Option<Provider>,
    pub country: Option<Country>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ShippingAddress {
    pub firstname: String,
    pub lastname: String,
    pub addressstreet: String,
    pub addresscity: String,
    pub addresspostalcode: String,
    pub addresscountry: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CreatePhysicalCardRequest {
    pub shipping_address: serde_json::Value,
}

/// Interface for ramps account operations.
#[spy_mock]
#[async_trait]
pub trait AccountsInterface: Send + Sync {
    /// Create or update an account for the authenticated wallet.
    async fn create_account(
        &self,
        wallet_id: Uuid,
        ip_address: String,
        request: AccountCreateRequest,
    ) -> Result<Account>;

    /// List all accounts associated with the wallet.
    async fn list_accounts(
        &self,
        wallet_id: Uuid,
        query: ListAccountsQuery,
    ) -> Result<Vec<Account>>;

    /// Retrieve the Rain services card for the wallet.
    async fn get_account_services_card(&self, wallet_id: Uuid) -> Result<Account>;

    /// Retrieve or create an account by provider for the wallet.
    async fn get_account_by_provider(&self, wallet_id: Uuid, provider: Provider)
    -> Result<Account>;

    /// Fetch account details by identifier, ensuring wallet ownership.
    async fn get_account_by_id(&self, wallet_id: Uuid, account_id: Uuid) -> Result<Account>;

    /// Update account by identifier.
    async fn update_account_by_id(
        &self,
        wallet_id: Uuid,
        account_id: Uuid,
        ip_address: String,
        request: AccountUpdateRequest,
    ) -> Result<Account>;

    /// Update account using provider/country lookup.
    async fn update_account_by_provider_and_country(
        &self,
        wallet_id: Uuid,
        provider: Provider,
        country: Option<Country>,
        ip_address: String,
        request: AccountUpdateRequest,
    ) -> Result<Account>;

    /// Create a physical card for the wallet.
    async fn create_physical_card(
        &self,
        wallet_id: Uuid,
        request: CreatePhysicalCardRequest,
    ) -> Result<Account>;
}
