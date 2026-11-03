use ace_engine::core::importer::{
    Importer, SitePackCreationStatus, SitePackFormat, SitePackId, SitePackSource, SitePackSourceId,
    SitePackSourceType,
};
use ace_engine::core::network_block::site_pack::site_pack_db::SitePackDB;
use ace_engine::settings::Settings;
use eyre::{Context, Result};
use futures_util::StreamExt;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

pub struct DefaultSitePacks {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub requires_pro: bool,
    pub sources: &'static [DefaultSitePacksSource],
}

#[derive(Clone)]
pub struct DefaultSitePacksSource {
    pub source: &'static str,
    pub source_type: SitePackSourceType,
    pub format: SitePackFormat,
}

// Keep this developer-only generator aligned with the Ace App catalog.
// The production offline exporter reads site-packs.json directly.
pub const DEFAULT_PACKS: &[DefaultSitePacks] = &[
    DefaultSitePacks {
        id: "pack-adult-free",
        name: "Adult Websites — Free",
        description: "Compact monthly adult-content list from ace-hosts.",
        requires_pro: false,
        sources: &[DefaultSitePacksSource {
            source: "https://github.com/Linea-Software/ace-hosts/releases/latest/download/adult.txt",
            source_type: SitePackSourceType::Url,
            format: SitePackFormat::Standard,
        }],
    },
    DefaultSitePacks {
        id: "pack-social-free",
        name: "Social Media Websites — Free",
        description: "Blocks social media sites.",
        requires_pro: false,
        sources: &[DefaultSitePacksSource {
            source: "https://github.com/Linea-Software/ace-hosts/releases/latest/download/social.txt",
            source_type: SitePackSourceType::Url,
            format: SitePackFormat::Standard,
        }],
    },
    DefaultSitePacks {
        id: "pack-gaming-free",
        name: "Gaming Websites — Free",
        description: "Blocks online gaming sites and services.",
        requires_pro: false,
        sources: &[DefaultSitePacksSource {
            source: "https://github.com/Linea-Software/ace-hosts/releases/latest/download/gaming.txt",
            source_type: SitePackSourceType::Url,
            format: SitePackFormat::Standard,
        }],
    },
    ///////////////
    // PRO LISTS //
    ///////////////
    DefaultSitePacks {
        id: "pack-adult-pro",
        name: "Adult Websites — Pro",
        description: "Extended monthly adult-content list from ace-hosts.",
        requires_pro: true,
        sources: &[DefaultSitePacksSource {
            source: "https://github.com/Linea-Software/ace-hosts/releases/latest/download/adult-extended.txt",
            source_type: SitePackSourceType::Url,
            format: SitePackFormat::Standard,
        }],
    },
    DefaultSitePacks {
        id: "pack-gambling-pro",
        name: "Gambling Websites — Pro",
        description: "Blocks online gambling sites.",
        requires_pro: true,
        sources: &[DefaultSitePacksSource {
            source: "https://github.com/Linea-Software/ace-hosts/releases/latest/download/gambling.txt",
            source_type: SitePackSourceType::Url,
            format: SitePackFormat::Standard,
        }],
    },
    DefaultSitePacks {
        id: "pack-messaging-pro",
        name: "Messaging Websites — Pro",
        description: "Blocks popular messaging applications.",
        requires_pro: true,
        sources: &[DefaultSitePacksSource {
            source: "https://github.com/Linea-Software/ace-hosts/releases/latest/download/messaging.txt",
            source_type: SitePackSourceType::Url,
            format: SitePackFormat::Standard,
        }],
    },
    DefaultSitePacks {
        id: "pack-streaming-pro",
        name: "Streaming Websites — Pro",
        description: "Blocks popular streaming services.",
        requires_pro: true,
        sources: &[DefaultSitePacksSource {
            source: "https://github.com/Linea-Software/ace-hosts/releases/latest/download/streaming.txt",
            source_type: SitePackSourceType::Url,
            format: SitePackFormat::Standard,
        }],
    },
    DefaultSitePacks {
        id: "pack-shopping-pro",
        name: "Shopping Websites — Pro",
        description: "Blocks popular shopping services.",
        requires_pro: true,
        sources: &[DefaultSitePacksSource {
            source: "https://github.com/Linea-Software/ace-hosts/releases/latest/download/shopping.txt",
            source_type: SitePackSourceType::Url,
            format: SitePackFormat::Standard,
        }],
    },
    DefaultSitePacks {
        id: "pack-anime-manga-pro",
        name: "Anime & Manga Websites — Pro",
        description: "Blocks anime, manga, manhwa, manhua, webtoons and related novel sites.",
        requires_pro: true,
        sources: &[DefaultSitePacksSource {
            source: "https://github.com/Linea-Software/ace-hosts/releases/latest/download/anime-manga.txt",
            source_type: SitePackSourceType::Url,
            format: SitePackFormat::Standard,
        }],
    },
    DefaultSitePacks {
        id: "pack-anime-manga-extended",
        name: "Anime & Manga Websites — Extended",
        description: "Optional broader anime, manga, manhwa, manhua, webtoon and novel blocking; may affect additional reading sites.",
        requires_pro: true,
        sources: &[DefaultSitePacksSource {
            source: "https://github.com/Linea-Software/ace-hosts/releases/latest/download/anime-manga-extended.txt",
            source_type: SitePackSourceType::Url,
            format: SitePackFormat::Standard,
        }],
    },
    DefaultSitePacks {
        id: "pack-forums-pro",
        name: "Forums & Communities — Pro",
        description: "Blocks discussion forums and community websites.",
        requires_pro: true,
        sources: &[DefaultSitePacksSource {
            source: "https://github.com/Linea-Software/ace-hosts/releases/latest/download/forums.txt",
            source_type: SitePackSourceType::Url,
            format: SitePackFormat::Standard,
        }],
    },
    DefaultSitePacks {
        id: "pack-news-pro",
        name: "News Websites — Pro",
        description: "Blocks news and online press websites.",
        requires_pro: true,
        sources: &[DefaultSitePacksSource {
            source: "https://github.com/Linea-Software/ace-hosts/releases/latest/download/news.txt",
            source_type: SitePackSourceType::Url,
            format: SitePackFormat::Standard,
        }],
    },
    DefaultSitePacks {
        id: "pack-sports-pro",
        name: "Sports Websites — Pro",
        description: "Blocks sports portals and result websites.",
        requires_pro: true,
        sources: &[DefaultSitePacksSource {
            source: "https://github.com/Linea-Software/ace-hosts/releases/latest/download/sports.txt",
            source_type: SitePackSourceType::Url,
            format: SitePackFormat::Standard,
        }],
    },
    DefaultSitePacks {
        id: "pack-dating-pro",
        name: "Dating Websites — Pro",
        description: "Blocks dating and matchmaking websites.",
        requires_pro: true,
        sources: &[DefaultSitePacksSource {
            source: "https://github.com/Linea-Software/ace-hosts/releases/latest/download/dating.txt",
            source_type: SitePackSourceType::Url,
            format: SitePackFormat::Standard,
        }],
    },
    DefaultSitePacks {
        id: "pack-ai-tools-pro",
        name: "AI Tools Websites — Pro",
        description: "Blocks AI chatbots and generative AI services.",
        requires_pro: true,
        sources: &[DefaultSitePacksSource {
            source: "https://github.com/Linea-Software/ace-hosts/releases/latest/download/ai-tools.txt",
            source_type: SitePackSourceType::Url,
            format: SitePackFormat::Standard,
        }],
    },
    DefaultSitePacks {
        id: "pack-phishing-pro",
        name: "Phishing Websites — Pro",
        description: "Blocks popular phishing sites.",
        requires_pro: true,
        sources: &[
            DefaultSitePacksSource {
                source: "https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/fakenews-only/hosts",
                source_type: SitePackSourceType::Url,
                format: SitePackFormat::Standard,
            },
            DefaultSitePacksSource {
                source: "https://raw.githubusercontent.com/olbat/ut1-blacklists/refs/heads/master/blacklists/phishing/domains",
                source_type: SitePackSourceType::Url,
                format: SitePackFormat::WildCardDomains,
            },
        ],
    },
    DefaultSitePacks {
        id: "pack-piracy-pro",
        name: "Piracy Websites — Pro",
        description: "Blocks popular piracy sites.",
        requires_pro: true,
        sources: &[DefaultSitePacksSource {
            source: "https://raw.githubusercontent.com/olbat/ut1-blacklists/refs/heads/master/blacklists/warez/domains",
            source_type: SitePackSourceType::Url,
            format: SitePackFormat::WildCardDomains,
        }],
    },
];

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .wrap_err("setting default subscriber failed")?;

    let site_packs_dir = Settings::get_site_packs_dir();
    std::fs::create_dir_all(site_packs_dir.join("premium"))
        .wrap_err("failed to create premium directory")?;
    std::fs::create_dir_all(site_packs_dir.join("free"))
        .wrap_err("failed to create free directory")?;

    for site_pack in DEFAULT_PACKS {
        let site_pack_id = SitePackId::try_new(site_pack.id.to_string())
            .map_err(|error| eyre::eyre!("Invalid built-in site-pack ID: {error}"))?;
        let mut output_dir = site_packs_dir.clone();
        output_dir.push(site_pack_id.to_string());

        if output_dir.exists() {
            std::fs::remove_dir_all(&output_dir)
                .wrap_err("failed to clean up existing directory")?;
        }
        std::fs::create_dir_all(&output_dir).wrap_err("failed to create output directory")?;

        let db_path = output_dir.join("pack.db");
        let db = SitePackDB::from(&db_path)
            .wrap_err_with(|| format!("failed to create site pack database at {:?}", db_path))?;

        let sources: Vec<_> = site_pack
            .sources
            .iter()
            .map(|source| SitePackSource {
                id: SitePackSourceId::from(Uuid::new_v4()),
                site_pack_id: site_pack_id.clone(),
                source: source.source.to_string(),
                source_type: source.source_type,
                format: source.format,
            })
            .collect();

        info!("Generating site pack: {}", site_pack.id);

        let mut stream = Importer::import_sources(&db, sources, std::env::temp_dir(), site_pack_id);

        while let Some(status) = stream.next().await {
            match status {
                SitePackCreationStatus::Started { .. } => {
                    info!("Import started for {}", site_pack.id)
                }
                SitePackCreationStatus::InProgress {
                    progress, phase, ..
                } => {
                    info!("Progress: {:.2}% ({phase})", progress);
                }
                SitePackCreationStatus::Completed { .. } => info!("Import completed"),
                SitePackCreationStatus::Failed { error, .. } => {
                    tracing::error!("Error during import: {error}");
                }
            }
        }
        info!("Database generated at {:?}", db_path);

        // Package the SQLite database file into a zip archive compressed with
        // zstd. The archive contains a single "pack.db" entry.
        let mut archive_file = site_packs_dir.clone();
        archive_file.push(if site_pack.requires_pro {
            "premium"
        } else {
            "free"
        });
        archive_file.push(format!("{}_bak.zip", site_pack.id));

        let archive_handle =
            std::fs::File::create(&archive_file).wrap_err("failed to create zip archive")?;
        let mut archive = zip::ZipWriter::new(archive_handle);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Zstd);
        archive
            .start_file("pack.db", options)
            .wrap_err("failed to start zip entry")?;

        let mut db_file = std::fs::File::open(&db_path).wrap_err("failed to open database")?;
        std::io::copy(&mut db_file, &mut archive).wrap_err("failed to write zip entry")?;
        archive
            .finish()
            .wrap_err("failed to finalize zip archive")?;

        info!(
            "[COMPLETED] Zip archive created at {}",
            archive_file.to_str().unwrap()
        );
    }

    Ok(())
}
