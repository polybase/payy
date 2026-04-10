INSERT INTO ramps_accounts (
    id,
    address,
    provider,
    external_id,
    kyc_status,
    kyc_update_required_fields,
    kyc_external_id,
    country,
    deposit_evm_address,
    withdraw_evm_address,
    metadata,
    added_at,
    updated_at,
    kyc_delegated_id,
    kyc_non_delegated_status,
    wallet_id
)
SELECT
    '00000000-0000-0000-0000-000000000001'::uuid,
    NULL,
    'PAYY',
    NULL,
    'APPROVED',
    NULL,
    NULL,
    NULL,
    NULL,
    NULL,
    '{"system": true, "purpose": "swap-liquidity"}'::jsonb,
    NOW(),
    NOW(),
    NULL,
    NULL,
    '00000000-0000-0000-0000-000000000000'::uuid
WHERE NOT EXISTS (
    SELECT 1
    FROM ramps_accounts
    WHERE wallet_id = '00000000-0000-0000-0000-000000000000'::uuid
      AND provider = 'PAYY'
      AND country IS NULL
);
