-- Restrict the replit support user from reading sensitive columns while retaining
-- access to non-sensitive data required by the admin dashboard.

DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'replit')
    THEN
        CREATE USER replit;
    END IF;
END
$$;

-- Remove existing broad table-level SELECT privileges on notes. We deliberately do not touch
-- INSERT privileges to avoid disrupting existing workflows.
REVOKE SELECT ON TABLE notes FROM replit;

-- Grant column-scoped SELECT on notes, excluding the private_key field.
GRANT SELECT (
    "id",
    "address",
    "psi",
    "value",
    "owner_id",
    "status",
    "parent_1_id",
    "received_ref_kind",
    "received_ref_id",
    "spent_at",
    "added_at",
    "updated_at",
    "commitment",
    "spend_ref_kind",
    "spend_ref_id",
    "parent_2_id",
    "version",
    "kind"
) ON TABLE notes TO replit;
