use std::time::Instant;

use axum::{Json, extract::State};
use barretenberg_api_interface::{ProveRequest, ProveResponse, VerifyRequest, VerifyResponse};
use contextful::ResultContextExt;
use tracing::info;

use crate::{
    error::HandlerError,
    extractors::{CustomJson, Permit},
    server::AppState,
};

pub(crate) async fn prove(
    State(state): State<AppState>,
    Permit(_permit): Permit,
    CustomJson(request): CustomJson<ProveRequest>,
) -> Result<Json<ProveResponse>, HandlerError> {
    let ProveRequest {
        program,
        bytecode,
        key,
        witness,
        oracle,
    } = request;

    let started_at = Instant::now();
    info!(
        target: "barretenberg_api_server",
        program_bytes = program.len(),
        bytecode_bytes = bytecode.len(),
        key_bytes = key.len(),
        witness_bytes = witness.len(),
        oracle,
        "prove request accepted"
    );

    let backend = state.backend();

    let proof = backend
        .prove(&program, &bytecode, &key, &witness, oracle)
        .await
        .context("backend prove request")?;

    let response = ProveResponse {
        proof: proof.into(),
    };
    info!(
        target: "barretenberg_api_server",
        proof_bytes = response.proof.len(),
        elapsed_ms = started_at.elapsed().as_millis() as u64,
        "generated proof successfully"
    );

    Ok(Json(response))
}

pub(crate) async fn verify(
    State(state): State<AppState>,
    CustomJson(request): CustomJson<VerifyRequest>,
) -> Result<Json<VerifyResponse>, HandlerError> {
    let VerifyRequest {
        proof,
        public_inputs,
        key,
        oracle,
    } = request;

    let started_at = Instant::now();
    info!(
        target: "barretenberg_api_server",
        proof_bytes = proof.len(),
        public_inputs_bytes = public_inputs.len(),
        key_bytes = key.len(),
        oracle,
        "verify request accepted"
    );

    let backend = state.backend();

    backend
        .verify(&proof, &public_inputs, &key, oracle)
        .await
        .context("backend verify request")?;

    let response = VerifyResponse { valid: true };
    info!(
        target: "barretenberg_api_server",
        elapsed_ms = started_at.elapsed().as_millis() as u64,
        "verified proof successfully"
    );

    Ok(Json(response))
}
