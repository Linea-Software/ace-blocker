use crates_llms_txt::CrateDocs;
use eyre::Result;
use std::fs;
use std::path::Path;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt().with_ansi(true).init();

    let crates = vec![
        "axum",
        "tokio",
        "diesel",
        "serde",
        "reqwest",
        "mitm-proxy",
        "chrono",
        "tracing",
        "axum-test",
        "schemars",
        "fake",
        "thiserror",
        "regex",
        "windows",
        "app_dirs2",
        "certkit",
    ];

    let output_dir = Path::new("docs/libs");
    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }

    for crate_name in crates {
        let llms_path = output_dir.join(format!("{}.llms.txt", crate_name));
        let full_path = output_dir.join(format!("{}.llms-full.txt", crate_name));

        if llms_path.exists() && full_path.exists() {
            info!(
                "Documentation for {} already exists. Skipping...",
                crate_name
            );
            continue;
        }

        info!("Fetching documentation for {}...", crate_name);

        match CrateDocs::from_online(crate_name, None).await {
            Ok(config) => {
                // Generate llms.txt
                let mut llms_txt = format!("# {} Documentation (LLMS)\n\n", crate_name);
                llms_txt.push_str("## API Overview\n\n");
                for item in &config.sessions {
                    llms_txt.push_str(&format!("### {}\n", item.title));
                    if !item.description.is_empty() {
                        llms_txt.push_str(&format!("{}\n", item.description));
                    }
                    llms_txt.push_str(&format!("Link: {}\n\n", item.link));
                }

                let llms_path = output_dir.join(format!("{}.llms.txt", crate_name));
                fs::write(&llms_path, llms_txt)?;
                info!("Saved {}", llms_path.display());

                // Generate llms-full.txt
                let mut full_txt = format!("# {} Full API Documentation\n\n", crate_name);
                for item in &config.full_sessions {
                    full_txt.push_str(&format!("## {}\n", item.link));
                    full_txt.push_str(&format!("{}\n\n", item.content));
                    full_txt.push_str("---\n\n");
                }

                let full_path = output_dir.join(format!("{}.llms-full.txt", crate_name));
                fs::write(&full_path, full_txt)?;
                info!("Saved {}", full_path.display());
            }
            Err(error) => {
                error!("Error fetching {}: {}", crate_name, error);
            }
        }
    }

    Ok(())
}
