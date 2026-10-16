//! Generate the OpenAPI spec JSON snapshot from the compiled utoipa definitions.
//!
//! Run with:
//!   cargo run --bin gen-openapi-spec
//!
//! Output: `docs/openapi.json` — committed to the repo so LLMs and tools
//! can inspect the full API surface without a running engine.

use utoipa::OpenApi;

fn main() {
    let spec = ace_engine::service::api::docs::ApiDoc::openapi();
    let json = serde_json::to_string_pretty(&spec).expect("Failed to serialize OpenAPI spec");

    let output_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/openapi.json");
    std::fs::write(&output_path, &json).expect("Failed to write OpenAPI spec file");

    eprintln!(
        "OpenAPI spec written to {} ({} bytes)",
        output_path.display(),
        json.len()
    );
}
