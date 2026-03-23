ALTER TABLE wallet_backups
    ADD COLUMN diff_of TEXT;

ALTER TABLE wallet_backups
    ADD CONSTRAINT fk_diff_of_last_update
    FOREIGN KEY (wallet_address, diff_of)
    REFERENCES wallet_backups(wallet_address, last_update);
