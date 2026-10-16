//! Standalone utility that installs the Ace Engine root CA certificate
//! into the system trust store.
//!
//! On **Windows** this requires Administrator privileges to add the
//! certificate to the Machine Root store.
//!
//! Usage:
//! ```text
//! cargo run --bin install-cert   # or just run the compiled executable
//! ```

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    println!("Ace Engine — Root CA Certificate Installer\n");

    let cert_dir = ace_engine::settings::Settings::get_cert_dir();
    let cert_path = cert_dir.join("ace_ca.pem");

    if cert_path.exists() {
        println!("Certificate already exists at {:?}", cert_path);
        println!(
            "To regenerate: delete the certs directory ({}), then re-run.",
            cert_dir.display()
        );
        return Ok(());
    }

    println!("Generating and installing root CA certificate...");

    match ace_engine::service::proxy::install_certificate() {
        Ok(()) => {
            println!();
            println!("SUCCESS: Root CA certificate installed.");
            println!("  Location: {:?}", cert_path);
            println!();
            #[cfg(target_os = "windows")]
            {
                println!("Note: Firefox users may need to enable");
                println!("'security.enterprise_roots.enabled' in about:config.");
            }
            println!("Note: You may need to restart your browser for the change to take effect.");
        }
        Err(error) => {
            eprintln!();
            eprintln!("FAILED to install certificate: {error}");

            #[cfg(target_os = "windows")]
            {
                eprintln!();
                eprintln!("On Windows this tool requires Administrator privileges.");
                eprintln!("Right-click the executable and select 'Run as administrator',");
                eprintln!("or run from an elevated terminal:");
                eprintln!("  install-cert.exe");
            }

            std::process::exit(1);
        }
    }

    Ok(())
}
