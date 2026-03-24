// lint-long-file-override allow-max-lines=300
#[cfg(feature = "diesel")]
use database::schema::ramps_methods;

#[cfg(feature = "diesel")]
use diesel::prelude::*;

use async_trait::async_trait;
use network::{Network, NetworkIdentifier};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use test_spy::spy_mock;
use uuid::Uuid;
use veil::Redact;

use crate::error::Result;
use crate::provider::Provider;

#[derive(Deserialize, Debug, Clone)]
pub struct ListMethodsQuery {
    pub provider: Option<Provider>,
    pub network: Option<Network>,
    pub account_id: Option<Uuid>,
    pub include_deleted: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MethodCreateRequest {
    pub network: Network,
    pub network_identifier: Option<NetworkIdentifier>,
    pub account_id: Uuid,
    pub local_id: String,
    pub set_as_default: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "diesel",
    derive(Queryable, Selectable, Identifiable, Insertable, AsChangeset)
)]
#[cfg_attr(feature = "diesel", diesel(primary_key(id)))]
#[cfg_attr(feature = "diesel", diesel(table_name = ramps_methods))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct Method {
    pub id: Uuid,
    pub account_id: Uuid,
    pub external_id: Option<String>,
    pub local_id: String,
    pub network: Network,
    pub network_identifier: NetworkIdentifier,
    pub preview: Option<NetworkIdentifier>,
    pub metadata: Option<Value>,
    pub is_default: bool,
    pub added_at: chrono::NaiveDateTime,
    pub frozen: bool,
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

impl Default for Method {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            account_id: Uuid::nil(),
            external_id: None,
            local_id: "method-default".to_string(),
            network: Network::Payy,
            network_identifier: NetworkIdentifier::default(),
            preview: None,
            metadata: None,
            is_default: false,
            added_at: chrono::Utc::now().naive_utc(),
            frozen: false,
            deleted_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "diesel", derive(Queryable, AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = ramps_methods))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct MethodUpdate {
    pub local_id: Option<String>,
    pub network: Option<Network>,
    pub network_identifier: Option<NetworkIdentifier>,
    pub preview: Option<NetworkIdentifier>,
    pub is_default: Option<bool>,
    pub metadata: Option<Value>,
}

impl MethodUpdate {
    #[must_use]
    pub fn without_metadata(mut self) -> Self {
        self.metadata = None;
        self
    }
}

#[derive(Clone, Deserialize, Redact)]
pub struct SetPinRequest {
    #[redact]
    pub pin: String,
}

#[derive(Serialize, Deserialize, Redact, Default)]
pub struct MethodPinResponse {
    #[redact]
    pub pin: String,
}

#[derive(Clone, Serialize, Deserialize, Redact, Default)]
pub struct MethodProviderSecretsResponse {
    #[redact]
    pub network_identifier: NetworkIdentifier,
}

#[spy_mock]
#[async_trait]
pub trait MethodsInterface: Send + Sync {
    async fn create_method(&self, wallet_id: Uuid, request: MethodCreateRequest) -> Result<Method>;

    async fn refresh_method(&self, method_id: Uuid) -> Result<Method>;

    async fn list_methods(&self, wallet_id: Uuid, query: ListMethodsQuery) -> Result<Vec<Method>>;

    async fn get_method(&self, wallet_id: Uuid, method_id: Uuid) -> Result<Method>;

    async fn delete_method(&self, wallet_id: Uuid, method_id: Uuid) -> Result<()>;

    async fn get_method_secrets(
        &self,
        wallet_id: Uuid,
        method_id: Uuid,
    ) -> Result<MethodProviderSecretsResponse>;

    async fn get_method_pin(&self, wallet_id: Uuid, method_id: Uuid) -> Result<MethodPinResponse>;

    async fn set_method_pin(
        &self,
        wallet_id: Uuid,
        method_id: Uuid,
        request: SetPinRequest,
    ) -> Result<MethodPinResponse>;

    async fn freeze_method(&self, wallet_id: Uuid, method_id: Uuid) -> Result<Method>;

    async fn unfreeze_method(&self, wallet_id: Uuid, method_id: Uuid) -> Result<Method>;
}
