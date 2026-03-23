use crate::{Error, NodeClientHttp, error::Result};
use client_http::{HttpBody, serde_to_query_params};
use http_interface::HttpMetadata;
use node_interface::{ListTxnsParams, ListTxnsResponse, TransactionRequest, TransactionResponse};
use reqwest::Method;
use serde::Serialize;
use zk_primitives::UtxoProof;

const TRANSACTION_PATH: &str = "/transaction";
const TRANSACTIONS_PATH: &str = "/transactions";

#[derive(Serialize)]
struct ListTxnsQuery {
    limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<String>,
    order: node_interface::ListTxnsOrder,
    poll: bool,
}

impl NodeClientHttp {
    /// Sends a transaction to Payy Network validator
    pub async fn transaction(&self, proof: UtxoProof) -> Result<TransactionResponse> {
        self.http_client
            .post(
                TRANSACTION_PATH,
                Some(HttpBody::json(TransactionRequest { proof })),
            )
            .exec()
            .await?
            .to_value()
            .await
    }

    /// List transactions from Payy Network validator.
    pub async fn list_transactions(&self, params: ListTxnsParams) -> Result<ListTxnsResponse> {
        let cursor = params
            .cursor
            .map(|cursor| cursor.serialize())
            .transpose()
            .map_err(|err| Error::SerdeJson(err.to_string(), transactions_metadata()))?;

        let query = ListTxnsQuery {
            limit: params.limit.min(100),
            cursor,
            order: params.order,
            poll: params.poll,
        };

        self.http_client
            .get(TRANSACTIONS_PATH)
            .query(serde_to_query_params(&query))
            .exec()
            .await?
            .to_value()
            .await
    }
}

fn transactions_metadata() -> HttpMetadata {
    HttpMetadata {
        method: Method::GET,
        path: TRANSACTIONS_PATH.to_owned(),
    }
}
