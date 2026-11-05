fn main() {
    // The Ace Server API client (src/ace_server_api) is generated explicitly
    // with `cargo run --bin gen-api-client` — never at build time. Earlier
    // versions fetched the OpenAPI spec from a live dev server during every
    // build, which made builds nondeterministic and dirtied the working tree
    // whenever a dev site happened to be running on localhost:5173.

    #[cfg(target_os = "windows")]
    {
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
