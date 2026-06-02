use httpmock::{Method::GET, MockServer};
use sourcify_interface::{ContractField, ContractLookup, Error, SourcifyClient};

use crate::{SourcifyReqwestClient, SourcifyReqwestClientOptions};

const ADDRESS: &str = "0x1111111111111111111111111111111111111111";

#[tokio::test]
async fn sends_contract_lookup_with_requested_fields() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/contract/1/{ADDRESS}"))
            .query_param("fields", "abi,creationMatch,runtimeMatch")
            .header("user-agent", "beam-cli sourcify-client-reqwest");
        then.status(200).json_body_obj(&serde_json::json!({
            "chainId": "1",
            "address": ADDRESS,
            "match": "exact_match",
            "creationMatch": null,
            "runtimeMatch": "match",
            "abi": [],
        }));
    });
    let client = test_client(&server);

    let response = client
        .contract(&ContractLookup {
            chain_id: 1,
            address: ADDRESS.to_owned(),
            fields: vec![
                ContractField::Abi,
                ContractField::Match,
                ContractField::CreationMatch,
                ContractField::RuntimeMatch,
            ],
            response_cap_bytes: 1024,
        })
        .await
        .expect("contract response");
    mock.assert();

    assert_eq!(response.contract.chain_id, "1");
    assert_eq!(response.contract.abi, Some(Vec::new()));
}

#[tokio::test]
async fn maps_unsupported_chain_status_body() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path(format!("/contract/999/{ADDRESS}"));
        then.status(404).body("unsupported chain");
    });
    let client = test_client(&server);

    let err = client
        .contract(&ContractLookup {
            chain_id: 999,
            address: ADDRESS.to_owned(),
            fields: vec![ContractField::Match],
            response_cap_bytes: 1024,
        })
        .await
        .expect_err("unsupported chain");
    mock.assert();

    assert!(matches!(err, Error::ChainUnsupported { chain_id: 999 }));
}

#[tokio::test]
async fn rejects_response_above_cap() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path(format!("/contract/1/{ADDRESS}"));
        then.status(200).body("x".repeat(32));
    });
    let client = test_client(&server);

    let err = client
        .contract(&ContractLookup {
            chain_id: 1,
            address: ADDRESS.to_owned(),
            fields: vec![ContractField::Match],
            response_cap_bytes: 8,
        })
        .await
        .expect_err("response cap");
    mock.assert();

    assert!(matches!(err, Error::ResponseTooLarge { cap_bytes: 8 }));
}

#[tokio::test]
async fn rejects_malformed_json() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path(format!("/contract/1/{ADDRESS}"));
        then.status(200).body("{not-json");
    });
    let client = test_client(&server);

    let err = client
        .contract(&ContractLookup {
            chain_id: 1,
            address: ADDRESS.to_owned(),
            fields: vec![ContractField::Match],
            response_cap_bytes: 1024,
        })
        .await
        .expect_err("malformed json");
    mock.assert();

    assert!(matches!(err, Error::MalformedResponse { .. }));
}

#[tokio::test]
async fn rejects_missing_requested_nullable_match_fields() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/contract/1/{ADDRESS}"))
            .query_param("fields", "creationMatch,runtimeMatch");
        then.status(200).json_body_obj(&serde_json::json!({
            "chainId": "1",
            "address": ADDRESS,
            "match": "exact_match",
            "creationMatch": null,
        }));
    });
    let client = test_client(&server);

    let err = client
        .contract(&ContractLookup {
            chain_id: 1,
            address: ADDRESS.to_owned(),
            fields: vec![
                ContractField::Match,
                ContractField::CreationMatch,
                ContractField::RuntimeMatch,
            ],
            response_cap_bytes: 1024,
        })
        .await
        .expect_err("missing runtimeMatch");
    mock.assert();

    assert!(matches!(err, Error::MalformedResponse { .. }));
}

fn test_client(server: &MockServer) -> SourcifyReqwestClient {
    SourcifyReqwestClient::new(SourcifyReqwestClientOptions::Custom {
        endpoint: server.base_url(),
    })
    .expect("test Sourcify client")
}
