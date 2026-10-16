use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SafeSearchEnforcement {
    DomainRedirection { target_host: String },
    QueryParameter { key: String, value: String },
}
