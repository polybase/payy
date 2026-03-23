ALTER TABLE wallet_backups
    DROP CONSTRAINT fk_diff_of_last_update;

ALTER TABLE wallet_backups
    DROP COLUMN diff_of;
