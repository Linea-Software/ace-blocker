// @generated automatically by Diesel CLI.

diesel::table! {
    accounts (table_lock) {
        table_lock -> Text,
        api_key -> Nullable<Text>,
        last_pro_check -> Nullable<Text>,
        tos_accepted_at -> Nullable<Text>,
        entitlement_token -> Nullable<Text>,
        entitlement_public_key -> Nullable<Text>,
        clock_high_watermark -> Nullable<Text>,
    }
}

diesel::table! {
    blocked_attempts (date) {
        date -> Text,
        attempts_count -> Integer,
    }
}

diesel::table! {
    network_block_exceptions (id) {
        id -> Text,
        network_block_id -> Text,
        rule -> Text,
    }
}

diesel::table! {
    network_block_locks (network_block_id) {
        network_block_id -> Text,
        lock_state -> Text,
        lock_end -> Nullable<Text>,
    }
}

diesel::table! {
    network_block_schedule_entries (id) {
        id -> Text,
        network_block_id -> Text,
        start_time -> Text,
        end_time -> Text,
        schedule_days_bitmask -> Integer,
    }
}

diesel::table! {
    network_block_to_site_pack (network_block_id, site_pack_id) {
        network_block_id -> Text,
        site_pack_id -> Text,
    }
}

diesel::table! {
    network_blocks (id) {
        id -> Text,
        name -> Text,
        enabled -> Bool,
        safe_search -> Bool,
        icon -> Nullable<Text>,
        schedule_timezone -> Text,
        schedule_mode -> Text,
    }
}

diesel::table! {
    proxy_exclusions (host) {
        host -> Text,
    }
}

diesel::table! {
    resistance_mode_settings (table_lock) {
        table_lock -> Text,
        enabled -> Bool,
        lock_state -> Text,
        lock_end -> Nullable<Text>,
        static_intensity -> Text,
        scripts_version -> Nullable<Text>,
    }
}

diesel::table! {
    safe_search_enforcements (domain) {
        domain -> Text,
        enforcement_type -> Text,
        target_host -> Nullable<Text>,
        query_param_key -> Nullable<Text>,
        query_param_value -> Nullable<Text>,
    }
}

diesel::table! {
    script_states (script_id) {
        script_id -> Text,
        enabled -> Bool,
    }
}

diesel::table! {
    settings (table_lock) {
        table_lock -> Text,
        is_pro -> Bool,
        is_seeded -> Bool,
        latest_update_check_at -> Nullable<Text>,
    }
}

diesel::table! {
    site_pack_sources (id) {
        id -> Text,
        site_pack_id -> Text,
        source -> Text,
        source_type -> Text,
        format -> Text,
    }
}

diesel::table! {
    site_packs (id) {
        id -> Text,
        remote_id -> Nullable<Text>,
        name -> Text,
        description -> Nullable<Text>,
        requires_pro -> Bool,
        last_updated_at -> Nullable<Text>,
        version -> Nullable<Text>,
        is_ignored -> Bool,
        icon -> Nullable<Text>,
    }
}

diesel::table! {
    user_defined_rules (id) {
        id -> Text,
        network_block_id -> Text,
        rule -> Text,
        rule_type -> Text,
    }
}

diesel::joinable!(network_block_exceptions -> network_blocks (network_block_id));
diesel::joinable!(network_block_locks -> network_blocks (network_block_id));
diesel::joinable!(network_block_schedule_entries -> network_blocks (network_block_id));
diesel::joinable!(network_block_to_site_pack -> network_blocks (network_block_id));
diesel::joinable!(network_block_to_site_pack -> site_packs (site_pack_id));
diesel::joinable!(site_pack_sources -> site_packs (site_pack_id));
diesel::joinable!(user_defined_rules -> network_blocks (network_block_id));

diesel::allow_tables_to_appear_in_same_query!(
    accounts,
    blocked_attempts,
    network_block_exceptions,
    network_block_locks,
    network_block_schedule_entries,
    network_block_to_site_pack,
    network_blocks,
    proxy_exclusions,
    resistance_mode_settings,
    safe_search_enforcements,
    script_states,
    settings,
    site_pack_sources,
    site_packs,
    user_defined_rules,
);
