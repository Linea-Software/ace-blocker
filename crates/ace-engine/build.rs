fn main() {
    // Recompile when the public entitlement verification key rotates.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=ACE_ENTITLEMENT_PUBLIC_KEY_BASE64");

    // A public release must never silently ship without the key needed to
    // validate Pro subscriptions. The private signing key stays on the server.
    if std::env::var("PROFILE").as_deref() == Ok("release") {
        let pinned_key = std::env::var("ACE_ENTITLEMENT_PUBLIC_KEY_BASE64")
            .expect("release builds require ACE_ENTITLEMENT_PUBLIC_KEY_BASE64");
        let raw = pinned_key.trim_end_matches('=');
        assert!(
            pinned_key.len() == 44
                && raw.len() == 43
                && raw
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'+' || byte == b'/'),
            "ACE_ENTITLEMENT_PUBLIC_KEY_BASE64 must be the base64 encoding of a raw 32-byte Ed25519 public key"
        );
    }

    // The Ace Server API client (src/ace_server_api) is generated explicitly
    // with `cargo run --bin gen-api-client` — never at build time. Earlier
    // versions fetched the OpenAPI spec from a live dev server during every
    // build, which made builds nondeterministic and dirtied the working tree
    // whenever a dev site happened to be running on localhost:5173.

    #[cfg(target_os = "windows")]
    {
        // Only embed the requireAdministrator manifest in actual binaries.
        // `CARGO_BIN_NAME` is unset for lib/test builds — applying the
        // manifest there made every test executable demand elevation, so
        // `cargo test` failed with os error 740 unless run from an elevated
        // shell. Binary behavior is unchanged: every bin keeps its manifest.
        if std::env::var("CARGO_BIN_NAME").is_ok() {
            ask_for_admin_permissions().unwrap();
        }
    }
}

#[cfg(target_os = "windows")]
fn ask_for_admin_permissions() -> std::io::Result<()> {
    use winres::WindowsResource;

    let mut res = WindowsResource::new();
    res.set_manifest(
        r#"
        <assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
            <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
                <security>
                    <requestedPrivileges>
                        <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
                    </requestedPrivileges>
                </security>
            </trustInfo>
        </assembly>
    "#,
    );
    res.compile()
}
