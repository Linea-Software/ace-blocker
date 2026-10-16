// @generated automatically by Diesel CLI.

diesel::table! {
    entries (host_hash) {
        host_hash -> BigInt,
    }
}

diesel::table! {
    pack_meta (id) {
        id -> Text,
        format_version -> Text,
        name -> Nullable<Text>,
        entry_count -> Integer,
        built_at -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(entries, pack_meta,);
