export type PayyClientErrorCode =
  | 'missing_privacy_signer'
  | 'missing_evm_submitter'
  | 'missing_evm_provider'
  | 'missing_raw_submitter'
  | 'chain_id_mismatch'
  | 'amount_zero'
  | 'commitment_mismatch'
  | 'nullifier_mismatch'
  | 'nonce_hash_mismatch'
  | 'note_spent'
  | 'invalid_claim_link'
  | 'invalid_incoming_transfer'
  | 'ephemeral_key_mismatch'
  | 'merkle_path_invalid'
  | 'commitment_not_found'
  | 'checkpoint_mismatch'
  | 'missing_owned_note'
  | 'prefix_mismatch'
  | 'privacy_account_mismatch'
  | 'evm_account_mismatch'
  | 'evm_recipient_zero'
  | 'field_out_of_range'
  | 'invalid_privacy_address'
  | 'value_out_of_range'
  | 'insufficient_balance'
  | 'transaction_reverted'
  | 'receipt_timeout'
  | 'contract_return_malformed'
  | 'proof_output_malformed'
  | 'missing_log_metadata'
  | 'invalid_hex'
  | 'fee_data_unavailable'
  | 'receipt_status_unknown'

export class PayyClientError extends Error {
  readonly code: PayyClientErrorCode
  readonly data?: unknown

  constructor(code: PayyClientErrorCode, message: string, data?: unknown) {
    super(message)
    this.code = code
    this.data = data
    this.name = 'PayyClientError'
  }
}

export function validationError(
  code: PayyClientErrorCode,
  message: string,
  data?: unknown
): PayyClientError {
  return new PayyClientError(code, message, data)
}
