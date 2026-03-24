ALTER TABLE ramps_transactions
ADD COLUMN from_note_kind TEXT,
ADD COLUMN to_note_kind TEXT;

UPDATE ramps_transactions
SET from_note_kind = CASE
        WHEN from_network = 'PAYY' THEN '000200000000000000893c499c542cef5e3811e1192ce70d8cc03d5c33590000'
        ELSE NULL
    END,
    to_note_kind = CASE
        WHEN to_network = 'PAYY' THEN '000200000000000000893c499c542cef5e3811e1192ce70d8cc03d5c33590000'
        ELSE NULL
    END;
