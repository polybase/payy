// @generated automatically by Diesel CLI.

diesel::table! {
    blocklist_ip (ip) {
        ip -> Text,
        data -> Jsonb,
        block -> Bool,
        request_count -> Int4,
        request_count_reset_at -> Timestamptz,
        request_count_total -> Int4,
        added_at -> Timestamptz,
    }
}

diesel::table! {
    blocklist_mobile (mobile) {
        mobile -> Text,
        reason -> Nullable<Text>,
        ip -> Nullable<Text>,
        block -> Bool,
        added_at -> Timestamptz,
        country_code -> Nullable<Text>,
    }
}

diesel::table! {
    blocklist_trace (id) {
        id -> Uuid,
        path -> Text,
        mobile -> Text,
        ip -> Nullable<Text>,
        added_at -> Timestamptz,
    }
}

diesel::table! {
    diagnostics (id) {
        id -> Uuid,
        address -> Text,
        backup_diffs -> Jsonb,
        state -> Jsonb,
        mnemonic -> Text,
        device_info -> Jsonb,
        message -> Nullable<Text>,
        added_at -> Timestamptz,
        wallet_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    faucets (id) {
        id -> Uuid,
        url -> Text,
        claimed_by -> Nullable<Text>,
        claimed_at -> Nullable<Timestamptz>,
        added_at -> Timestamptz,
        device_id -> Nullable<Text>,
    }
}

diesel::table! {
    ip_data (id) {
        id -> Uuid,
        ip -> Text,
        data -> Jsonb,
        added_at -> Timestamptz,
    }
}

diesel::table! {
    migrate_elements (element) {
        element -> Text,
    }
}

diesel::table! {
    nfts (id) {
        id -> Uuid,
        url -> Text,
        price -> Int4,
        payment_id -> Nullable<Uuid>,
        claimed_by -> Nullable<Text>,
        claimed_at -> Nullable<Timestamptz>,
        added_at -> Timestamptz,
    }
}

diesel::table! {
    notes (id) {
        id -> Uuid,
        address -> Text,
        private_key -> Text,
        psi -> Text,
        value -> Numeric,
        owner_id -> Uuid,
        status -> Text,
        parent_1_id -> Nullable<Uuid>,
        received_ref_kind -> Nullable<Text>,
        received_ref_id -> Nullable<Text>,
        spent_at -> Nullable<Timestamptz>,
        added_at -> Timestamptz,
        updated_at -> Timestamptz,
        commitment -> Text,
        spend_ref_kind -> Nullable<Text>,
        spend_ref_id -> Nullable<Text>,
        parent_2_id -> Nullable<Uuid>,
        version -> Int2,
        kind -> Text,
    }
}

diesel::table! {
    payments (id) {
        id -> Uuid,
        product -> Text,
        provider -> Text,
        external_id -> Nullable<Text>,
        data -> Jsonb,
        amount -> Int4,
        currency -> Text,
        status -> Text,
        payment_by -> Nullable<Text>,
        added_at -> Timestamptz,
    }
}

diesel::table! {
    ramps_accounts (id) {
        id -> Uuid,
        address -> Nullable<Text>,
        provider -> Text,
        external_id -> Nullable<Text>,
        kyc_status -> Text,
        kyc_update_required_fields -> Nullable<Jsonb>,
        kyc_external_id -> Nullable<Text>,
        country -> Nullable<Text>,
        deposit_evm_address -> Nullable<Text>,
        withdraw_evm_address -> Nullable<Text>,
        metadata -> Nullable<Jsonb>,
        added_at -> Timestamptz,
        updated_at -> Timestamptz,
        kyc_delegated_id -> Nullable<Uuid>,
        kyc_non_delegated_status -> Nullable<Text>,
        wallet_id -> Uuid,
    }
}

diesel::table! {
    ramps_events (id) {
        id -> Uuid,
        provider -> Text,
        data -> Jsonb,
        source -> Text,
        added_at -> Timestamptz,
        path -> Nullable<Text>,
        transaction_id -> Nullable<Text>,
        account_id -> Nullable<Text>,
        success -> Nullable<Bool>,
    }
}

diesel::table! {
    ramps_methods (id) {
        id -> Uuid,
        account_id -> Uuid,
        external_id -> Nullable<Text>,
        local_id -> Text,
        network -> Text,
        network_identifier -> Jsonb,
        preview -> Nullable<Jsonb>,
        metadata -> Nullable<Jsonb>,
        is_default -> Bool,
        added_at -> Timestamptz,
        frozen -> Bool,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    ramps_quotes (id) {
        id -> Uuid,
        provider -> Text,
        account_id -> Uuid,
        external_id -> Text,
        from_currency -> Text,
        from_amount -> Numeric,
        from_network -> Text,
        to_currency -> Text,
        to_amount -> Numeric,
        to_network -> Text,
        metadata -> Nullable<Jsonb>,
        expires_at -> Timestamptz,
        added_at -> Timestamptz,
    }
}

diesel::table! {
    ramps_transactions (id) {
        id -> Uuid,
        address -> Nullable<Text>,
        provider -> Text,
        account_id -> Uuid,
        external_id -> Nullable<Text>,
        external_fund_id -> Nullable<Text>,
        local_id -> Nullable<Text>,
        quote_id -> Nullable<Uuid>,
        status -> Text,
        from_currency -> Text,
        from_amount -> Numeric,
        from_network -> Text,
        from_network_identifier -> Nullable<Jsonb>,
        to_currency -> Text,
        to_amount -> Numeric,
        to_network -> Text,
        to_network_identifier -> Nullable<Jsonb>,
        evm_address -> Nullable<Text>,
        name -> Nullable<Text>,
        memo -> Nullable<Text>,
        desc -> Nullable<Text>,
        emoji -> Nullable<Text>,
        category -> Text,
        metadata -> Nullable<Jsonb>,
        transaction_at -> Nullable<Timestamptz>,
        added_at -> Timestamptz,
        updated_at -> Timestamptz,
        icon -> Nullable<Text>,
        pending_refund_amount -> Nullable<Numeric>,
        funding_status -> Nullable<Text>,
        funding_due_amount -> Nullable<Numeric>,
        wallet_id -> Uuid,
        status_reason -> Nullable<Text>,
        private_key -> Nullable<Text>,
        funding_kind -> Text,
        from_note_kind -> Nullable<Text>,
        to_note_kind -> Nullable<Text>,
    }
}

diesel::table! {
    registry_notes (id) {
        id -> Uuid,
        block -> Int8,
        public_key -> Text,
        encrypted_key -> Bytea,
        encrypted_note -> Bytea,
        added_at -> Timestamptz,
    }
}

diesel::table! {
    rewards (wallet_id) {
        address -> Nullable<Text>,
        code -> Text,
        points -> Int4,
        invites -> Int4,
        claims -> Jsonb,
        prize -> Nullable<Text>,
        added_at -> Timestamptz,
        updated_at -> Timestamptz,
        wallet_id -> Uuid,
    }
}

diesel::table! {
    rewards_invites (from_wallet_id, to_wallet_id) {
        from_address -> Nullable<Text>,
        to_address -> Nullable<Text>,
        added_at -> Timestamptz,
        from_wallet_id -> Uuid,
        to_wallet_id -> Uuid,
    }
}

diesel::table! {
    rewards_points (id) {
        id -> Uuid,
        address -> Nullable<Text>,
        reason -> Text,
        points -> Int4,
        added_at -> Timestamptz,
        wallet_id -> Uuid,
    }
}

diesel::table! {
    support_canned_response_tags (support_canned_response_id, support_tag_id) {
        support_canned_response_id -> Uuid,
        support_tag_id -> Uuid,
        added_at -> Timestamptz,
    }
}

diesel::table! {
    support_canned_responses (id) {
        id -> Uuid,
        name -> Text,
        display_name -> Text,
        content -> Text,
        is_active -> Bool,
        added_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    support_issue_tags (support_issue_id, support_tag_id) {
        support_issue_id -> Uuid,
        support_tag_id -> Uuid,
        added_at -> Timestamptz,
    }
}

diesel::table! {
    support_issues (id) {
        id -> Uuid,
        wallet_id -> Uuid,
        external_id -> Uuid,
        status -> Text,
        channel -> Text,
        subject -> Nullable<Text>,
        unread_count -> Int4,
        last_message -> Text,
        last_message_at -> Timestamptz,
        last_read_at -> Nullable<Timestamptz>,
        closed_at -> Nullable<Timestamptz>,
        updated_at -> Timestamptz,
        added_at -> Timestamptz,
        metadata -> Nullable<Jsonb>,
        auto_close_at -> Nullable<Timestamptz>,
        auto_close_minutes -> Nullable<Int4>,
        last_ai_support_bot_processed_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    support_messages (id) {
        id -> Uuid,
        support_issue_id -> Uuid,
        external_id -> Text,
        emoji -> Nullable<Text>,
        role -> Text,
        message -> Text,
        attachment -> Nullable<Jsonb>,
        added_at -> Timestamptz,
        is_bot -> Bool,
        is_internal -> Bool,
        agent_id -> Nullable<Int4>,
    }
}

diesel::table! {
    support_tags (id) {
        id -> Uuid,
        name -> Text,
        display_name -> Text,
        color -> Text,
        is_active -> Bool,
        added_at -> Timestamptz,
        auto_close_minutes -> Nullable<Int4>,
    }
}

diesel::table! {
    token_price_history (id) {
        id -> Uuid,
        #[max_length = 20]
        symbol -> Nullable<Varchar>,
        #[max_length = 50]
        network -> Varchar,
        contract_address -> Nullable<Text>,
        price -> Numeric,
        currency -> Text,
        last_updated_at -> Timestamptz,
        fetched_at -> Timestamptz,
        metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    token_prices (id) {
        id -> Uuid,
        #[max_length = 20]
        symbol -> Nullable<Varchar>,
        #[max_length = 50]
        network -> Varchar,
        contract_address -> Nullable<Text>,
        price -> Numeric,
        currency -> Text,
        last_updated_at -> Timestamptz,
        fetched_at -> Timestamptz,
        metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    udh_referral_links (posthog_id) {
        posthog_id -> Text,
        user_device_hash -> Text,
        referrer_url -> Text,
        claimed_at -> Nullable<Timestamptz>,
        wallet_id -> Nullable<Uuid>,
        added_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    wallet_activity (id) {
        id -> Uuid,
        address -> Text,
        kind -> Text,
        data -> Bytea,
        active -> Bool,
        completed_at -> Nullable<Timestamptz>,
        updated_at -> Timestamptz,
        added_at -> Timestamptz,
    }
}

diesel::table! {
    wallet_auths (id) {
        id -> Uuid,
        wallet_id -> Uuid,
        kind -> Text,
        value -> Text,
        enabled -> Bool,
        added_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    wallet_backup_tags (wallet_address, tag) {
        wallet_address -> Text,
        tag -> Text,
        last_update -> Text,
    }
}

diesel::table! {
    wallet_backups (wallet_address, last_update) {
        wallet_address -> Text,
        last_update -> Text,
        backup_path -> Text,
        backup_hash -> Bytea,
        added_at -> Timestamptz,
        diff_of -> Nullable<Text>,
    }
}

diesel::table! {
    wallet_notes (commitment) {
        commitment -> Text,
        address -> Text,
        data -> Bytea,
        status -> Text,
        activity_id -> Nullable<Uuid>,
        updated_at -> Timestamptz,
        added_at -> Timestamptz,
    }
}

diesel::table! {
    wallets (id) {
        id -> Uuid,
        address -> Text,
        expo_push_token -> Nullable<Text>,
        deposit_address -> Nullable<Text>,
        added_at -> Timestamptz,
        updated_at -> Timestamptz,
        atlas_customer_id -> Nullable<Text>,
        kyc -> Nullable<Jsonb>,
        country -> Nullable<Text>,
        language -> Nullable<Text>,
        ip_country -> Nullable<Text>,
        data -> Nullable<Jsonb>,
        version -> Int2,
        fraud_block -> Bool,
    }
}

diesel::table! {
    wallets_addresses (address) {
        address -> Text,
        wallet_id -> Uuid,
    }
}

diesel::joinable!(support_canned_response_tags -> support_canned_responses (support_canned_response_id));
diesel::joinable!(support_canned_response_tags -> support_tags (support_tag_id));
diesel::joinable!(support_issue_tags -> support_issues (support_issue_id));
diesel::joinable!(support_issue_tags -> support_tags (support_tag_id));
diesel::joinable!(wallet_auths -> wallets (wallet_id));

diesel::allow_tables_to_appear_in_same_query!(
    blocklist_ip,
    blocklist_mobile,
    blocklist_trace,
    diagnostics,
    faucets,
    ip_data,
    migrate_elements,
    nfts,
    notes,
    payments,
    ramps_accounts,
    ramps_events,
    ramps_methods,
    ramps_quotes,
    ramps_transactions,
    registry_notes,
    rewards,
    rewards_invites,
    rewards_points,
    support_canned_response_tags,
    support_canned_responses,
    support_issue_tags,
    support_issues,
    support_messages,
    support_tags,
    token_price_history,
    token_prices,
    udh_referral_links,
    wallet_activity,
    wallet_auths,
    wallet_backup_tags,
    wallet_backups,
    wallet_notes,
    wallets,
    wallets_addresses,
);
