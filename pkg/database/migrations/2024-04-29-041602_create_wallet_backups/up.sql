CREATE TABLE wallet_backups (
    wallet_address char(64) NOT NULL,
    last_update text NOT NULL,
    backup_path text NOT NULL,
    backup_hash bytea NOT NULL,
    added_at timestamp with time zone NOT NULL DEFAULT now(),

    PRIMARY KEY (wallet_address, last_update)
);

CREATE TABLE wallet_backup_tags (
    wallet_address char(64) NOT NULL,
    tag text NOT NULL,
    last_update text NOT NULL,

    PRIMARY KEY (wallet_address, tag),
    FOREIGN KEY (wallet_address, last_update) REFERENCES wallet_backups (wallet_address, last_update)
);
