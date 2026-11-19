use crate::{ace_server_api::HttpClient, core::ApiKey};

pub struct AceServerClient {}

impl AceServerClient {
    /// Builds the engine's HTTP client for the Ace server (base URL depends
    /// on the build profile). Named `new_http_client` — not `new` — because
    /// it returns a different type than `Self`.
    pub fn base_url() -> &'static str {
        #[cfg(debug_assertions)]
        {
            "http://localhost:5173"
        }
        #[cfg(not(debug_assertions))]
        {
            "https://aceblocker.com"
        }
    }

    pub fn new_http_client(api_key: Option<&ApiKey>) -> HttpClient {
        let client = HttpClient::new().with_base_url(Self::base_url());
        if let Some(api_key) = api_key {
            client.with_header("x-api-key", api_key.as_ref().to_owned())
        } else {
            client
        }
    }
}
