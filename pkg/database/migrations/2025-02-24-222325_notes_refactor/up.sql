ALTER TABLE notes DROP CONSTRAINT IF EXISTS notes_claim_id_key;

ALTER TABLE "notes" RENAME COLUMN "claim_reason" TO "received_ref_kind";
ALTER TABLE "notes" RENAME COLUMN "claim_id" TO "received_ref_id";
ALTER TABLE "notes" RENAME COLUMN "claimed_at" TO "spent_at";
ALTER TABLE "notes" RENAME COLUMN "parent_id" TO "parent_1_id";

ALTER TABLE "notes" ADD COLUMN "spend_ref_kind" TEXT;
ALTER TABLE "notes" ADD COLUMN "spend_ref_id" UUID;
ALTER TABLE "notes" ADD COLUMN "parent_2_id" UUID;


-- Update status from READY to UNSPENT
UPDATE notes
SET status = 'UNSPENT'
WHERE status = 'READY';

-- Update status from ASSIGNED to TXN_INPUT_ASSIGNED
UPDATE notes
SET status = 'TXN_INPUT_ASSIGNED'
WHERE status = 'ASSIGNED';

-- Update status from PENDING to TXN_OUTPUT_PENDING
UPDATE notes
SET status = 'TXN_OUTPUT_PENDING'
WHERE status = 'PENDING';

-- Update owner_id where owner_id is '00000000-0000-0000-0000-000000000000' and claim_reason is not 'NFT'
UPDATE notes
SET owner_id = '00000000-0000-0000-0000-000000000001'
WHERE owner_id = '00000000-0000-0000-0000-000000000000' AND received_ref_kind != 'NFT';

-- Update parent records based on received_ref_kind and received_ref_id
UPDATE notes AS parent
SET spend_ref_kind = child.received_ref_kind,
    spend_ref_id = child.received_ref_id
FROM notes AS child
WHERE (child.received_ref_kind = 'RAMP_TRANSACTION' OR child.received_ref_kind = 'NFT')
  AND child.status != 'DROPPED'
  AND parent.id = child.parent_1_id;

-- Update sibling records with the same parent_id
WITH original_child AS (
  SELECT parent_1_id, received_ref_kind, received_ref_id
  FROM notes
  WHERE (received_ref_kind = 'RAMP_TRANSACTION' OR received_ref_kind = 'NFT')
    AND status != 'DROPPED'
)
UPDATE notes AS sibling
SET received_ref_kind = original_child.received_ref_kind,
    received_ref_id = original_child.received_ref_id
FROM original_child
WHERE sibling.parent_1_id = original_child.parent_1_id
  AND sibling.status != 'DROPPED';

