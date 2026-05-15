use crate::{
    error::{Error, Result},
    privacy_config::{
        PRIVACY_BRIDGE_ADDRESS, PRIVACY_DEPLOYMENT_PREDEPLOY, PRIVACY_PROVER_BB_CLI,
        PRIVACY_STANDARD_ID, PRIVACY_STANDARD_VERSION, PRIVACY_STATE_POLICY_PERSISTED,
        PRIVACY_TOKEN_POLICY_ERC20_POOLS, PrivacyFeatureFlags, PrivacyProfile,
    },
};

#[derive(Debug)]
pub(super) struct PrivacyProfileArgs {
    pub bridge: Option<String>,
    pub deployment: Option<String>,
    pub features: Option<String>,
    pub prover: Option<String>,
    pub standard: Option<String>,
    pub state_policy: Option<String>,
    pub token_policy: Option<String>,
    pub version: Option<u32>,
}

pub(super) fn build_privacy_profile(args: PrivacyProfileArgs) -> Result<Option<PrivacyProfile>> {
    let requested = args.bridge.is_some()
        || args.deployment.is_some()
        || args.prover.is_some()
        || args.standard.is_some()
        || args.state_policy.is_some()
        || args.token_policy.is_some()
        || args.version.is_some()
        || args.features.is_some();

    if !requested {
        return Ok(None);
    }

    let profile = PrivacyProfile {
        standard: args
            .standard
            .unwrap_or_else(|| PRIVACY_STANDARD_ID.to_string()),
        version: args.version.unwrap_or(PRIVACY_STANDARD_VERSION),
        bridge: args
            .bridge
            .unwrap_or_else(|| PRIVACY_BRIDGE_ADDRESS.to_string()),
        deployment: args
            .deployment
            .unwrap_or_else(|| PRIVACY_DEPLOYMENT_PREDEPLOY.to_string()),
        prover: args
            .prover
            .unwrap_or_else(|| PRIVACY_PROVER_BB_CLI.to_string()),
        token_policy: args
            .token_policy
            .unwrap_or_else(|| PRIVACY_TOKEN_POLICY_ERC20_POOLS.to_string()),
        state_policy: args
            .state_policy
            .unwrap_or_else(|| PRIVACY_STATE_POLICY_PERSISTED.to_string()),
        features: parse_privacy_features(args.features)?,
    };
    profile.validate()?;
    Ok(Some(profile))
}

pub(super) fn privacy_status(profile: Option<&PrivacyProfile>) -> String {
    profile.map_or_else(
        || "-".to_string(),
        |profile| format!("{}:v{}", profile.standard, profile.version),
    )
}

fn parse_privacy_features(features: Option<String>) -> Result<PrivacyFeatureFlags> {
    let Some(features) = features else {
        return Ok(PrivacyFeatureFlags::default());
    };
    let mut flags = PrivacyFeatureFlags {
        mint: false,
        burn: false,
        direct_send: false,
        incoming: false,
        claim: false,
        claim_links: false,
        ephemeral: false,
        private_payments: false,
    };

    for feature in features
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        match feature {
            "all" => return Ok(PrivacyFeatureFlags::default()),
            "mint" => flags.mint = true,
            "burn" => flags.burn = true,
            "direct-send" | "direct_send" => flags.direct_send = true,
            "incoming" => flags.incoming = true,
            "claim" => flags.claim = true,
            "claim-links" | "claim_links" => flags.claim_links = true,
            "ephemeral" => flags.ephemeral = true,
            "private-payments" | "private_payments" => flags.private_payments = true,
            _ => {
                return Err(Error::InvalidPrivacyFeature {
                    feature: feature.to_string(),
                });
            }
        }
    }

    Ok(flags)
}
