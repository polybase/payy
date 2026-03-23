-- Restore the original broad SELECT privileges for the replit user.

REVOKE SELECT (
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
) ON TABLE notes FROM replit;
GRANT SELECT ON TABLE notes TO replit;
DROP USER replit;
