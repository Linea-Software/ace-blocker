//! Regenerates the committed OpenAPI snapshot (`docs/openapi.json`) without
//! requiring elevation.
//!
//! The `gen-openapi-spec` bin carries the requireAdministrator manifest (like
//! every engine binary), which makes it unusable in unelevated shells and CI.
//! This test is the unelevated equivalent:
//!
//! ```text
//! cargo test --test gen_openapi_snapshot -- --ignored
//! ```
//!
//! It writes the same file the bin writes: `docs/openapi.json`.

use utoipa::OpenApi;

#[test]
#[ignore = "run explicitly to regenerate docs/openapi.json"]
fn regenerate_openapi_snapshot() {
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
