// lint-long-file-override allow-max-lines=300
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use currency::Currency;
#[cfg(feature = "diesel")]
use database::schema::ramps_quotes;
#[cfg(feature = "diesel")]
use diesel::prelude::*;
use element::Element;
use network::Network;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use test_spy::spy_mock;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::provider::Provider;

#[derive(Serialize, Deserialize)]
pub struct QuoteRequestAmount {
    pub currency: Currency,
    pub amount: Option<Element>,
    pub network: Network,
}

#[derive(Serialize, Deserialize)]
pub struct QuoteResponseAmount {
    pub currency: Currency,
    pub amount: Element,
    pub network: Network,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuoteProviderResponse {
    Estimated(QuoteProviderResponseEstimated),
    Guaranteed(QuoteProviderResponseGuarenteed),
}

pub enum QuoteRequestDefinedAmount {
    From(Element),
    To(Element),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QuoteRequest {
    pub provider: Option<Provider>,
    pub from_currency: Currency,
    pub from_amount: Option<Element>,
    pub from_network: Network,
    pub to_currency: Currency,
    pub to_amount: Option<Element>,
    pub to_network: Network,
}

impl QuoteRequest {
    #[must_use]
    pub fn is_onramp(&self) -> bool {
        self.to_network == Network::Payy
    }

    #[must_use]
    pub fn is_offramp(&self) -> bool {
        self.from_network == Network::Payy
    }

    #[must_use]
    pub fn get_alt_currency(&self) -> Currency {
        if self.is_onramp() {
            self.from_currency
        } else {
            self.to_currency
        }
    }

    /// Determines which amount is provided in the request.
    ///
    /// # Errors
    ///
    /// Returns [`Error::QuoteMissingBothAmounts`] when neither `from_amount` nor `to_amount` is set.
    pub fn get_defined_amount(&self) -> Result<QuoteRequestDefinedAmount> {
        match (self.from_amount, self.to_amount) {
            (Some(from), _) => Ok(QuoteRequestDefinedAmount::From(from)),
            (_, Some(to)) => Ok(QuoteRequestDefinedAmount::To(to)),
            (None, None) => Err(Error::QuoteMissingBothAmounts),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteProviderResponseEstimated {
    pub provider: Provider,
    pub from_amount: Element,
    pub to_amount: Element,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteProviderResponseGuarenteed {
    pub id: String,
    pub provider: Provider,
    pub from_amount: Element,
    pub to_amount: Element,
    pub expires_at: DateTime<Utc>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "diesel",
    derive(Queryable, Selectable, Identifiable, Insertable, AsChangeset)
)]
#[cfg_attr(feature = "diesel", diesel(primary_key(id)))]
#[cfg_attr(feature = "diesel", diesel(table_name = ramps_quotes))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct Quote {
    pub id: Uuid,
    pub provider: Provider,
    pub account_id: Uuid,
    pub external_id: String,
    pub from_currency: Currency,
    pub from_amount: Element,
    pub from_network: Network,
    pub to_currency: Currency,
    pub to_amount: Element,
    pub to_network: Network,
    pub metadata: Option<Value>,
    pub expires_at: DateTime<Utc>,
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(default)]
pub struct QuoteResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,
    pub provider: Provider,
    pub account_id: Uuid,
    pub from_currency: Currency,
    pub from_amount: Element,
    pub from_network: Network,
    pub to_currency: Currency,
    pub to_amount: Element,
    pub to_network: Network,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_at: Option<DateTime<Utc>>,
}

impl Default for QuoteResponse {
    fn default() -> Self {
        Self {
            id: None,
            provider: Provider::Alfred,
            account_id: Uuid::nil(),
            from_currency: Currency::USD,
            from_amount: Element::default(),
            from_network: Network::Payy,
            to_currency: Currency::USD,
            to_amount: Element::default(),
            to_network: Network::Payy,
            metadata: None,
            expires_at: None,
            added_at: None,
        }
    }
}

impl QuoteResponse {
    #[must_use]
    pub fn from_quote_db(quote: Quote) -> Self {
        Self {
            id: Some(quote.id),
            provider: quote.provider,
            account_id: quote.account_id,
            from_currency: quote.from_currency,
            from_amount: quote.from_amount,
            from_network: quote.from_network,
            to_currency: quote.to_currency,
            to_amount: quote.to_amount,
            to_network: quote.to_network,
            metadata: quote.metadata,
            expires_at: Some(quote.expires_at),
            added_at: Some(quote.added_at),
        }
    }

    #[must_use]
    pub fn from_estimated_response(
        response: QuoteProviderResponseEstimated,
        request: &QuoteRequest,
        account_id: Uuid,
    ) -> Self {
        Self {
            id: None,
            account_id,
            provider: response.provider,
            from_currency: request.from_currency,
            from_amount: response.from_amount,
            from_network: request.from_network,
            to_currency: request.to_currency,
            to_amount: response.to_amount,
            to_network: request.to_network,
            metadata: response.metadata,
            expires_at: None,
            added_at: None,
        }
    }
}

impl Quote {
    #[must_use]
    pub fn is_onramp(&self) -> bool {
        self.to_network == Network::Payy
    }

    #[must_use]
    pub fn is_offramp(&self) -> bool {
        self.from_network == Network::Payy
    }
}

#[spy_mock]
#[async_trait]
pub trait QuotesInterface: Send + Sync {
    async fn create_quote(&self, wallet_id: Uuid, request: QuoteRequest) -> Result<QuoteResponse>;
}
