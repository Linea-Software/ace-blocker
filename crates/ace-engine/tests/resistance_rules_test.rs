//! Self-contained test for [`InjectableScriptTree::load_from_json`] and the
//! reverse-domain lookup/inheritance logic.
//!
//! The old version read `.temp/scripts-data.json` from a developer machine
//! and silently skipped when it was missing — always passing vacuously in CI
//! (TESTINFRA-02). This version builds its own catalog + script files.

use ace_engine::core::common::injectable_scripts::InjectableScriptTree;
use std::io::Write;
use std::path::Path;

/// Writes a catalog with script entries for `youtube.com` and `twitch.tv`
/// (reverse-domain keys `com/youtube` and `tv/twitch`) plus the referenced
/// script files.
fn write_fixture(dir: &Path) {
    std::fs::create_dir_all(dir.join("com/youtube")).unwrap();
    std::fs::create_dir_all(dir.join("tv/twitch")).unwrap();
    std::fs::write(
        dir.join("com/youtube/grayscale.js"),
        "console.log('grayscale');",
    )
    .unwrap();
    std::fs::write(dir.join("tv/twitch/captcha.js"), "console.log('captcha');").unwrap();

    let catalog = r#"{
        "sites": {
            "com": {
                "scripts": [],
                "children": {
                    "youtube": {
                        "scripts": [
                            {
                                "id": "youtube-grayscale",
                                "path": "com/youtube/grayscale.js",
                                "name": "Grayscale",
                                "description": "desaturate youtube",
                                "version": "1.0"
                            }
                        ],
                        "children": {}
                    }
                }
            },
            "tv": {
                "scripts": [],
                "children": {
                    "twitch": {
                        "scripts": [
                            {
                                "id": "twitch-captcha",
                                "path": "tv/twitch/captcha.js",
                                "name": "Captcha",
                                "description": "fake captcha on twitch",
                                "version": "1.0"
                            }
                        ],
                        "children": {}
                    }
                }
            }
        }
    }"#;

    let mut file = std::fs::File::create(dir.join("scripts-data.json")).unwrap();
    file.write_all(catalog.as_bytes()).unwrap();
}

#[test]
fn test_load_custom_rules_from_json() {
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path());

    let json_path = temp.path().join("scripts-data.json");
    let tree = InjectableScriptTree::load_from_json(&json_path).expect("Failed to load JSON");

    // Direct host lookup.
    let youtube_scripts = tree.get_scripts("youtube.com");
    assert_eq!(youtube_scripts.len(), 1, "youtube.com should have 1 script");
    assert!(youtube_scripts[0].contains("grayscale"));

    // Inheritance: www.youtube.com resolves through com → youtube.
    let www_youtube_scripts = tree.get_scripts("www.youtube.com");
    assert_eq!(
        www_youtube_scripts.len(),
        1,
        "www.youtube.com should inherit youtube.com scripts"
    );

    // twitch.tv under tv → twitch.
    let twitch_scripts = tree.get_scripts("twitch.tv");
    assert_eq!(twitch_scripts.len(), 1, "twitch.tv should have 1 script");
    assert!(twitch_scripts[0].contains("captcha"));

    // Unrelated host gets nothing.
    assert!(tree.get_scripts("example.org").is_empty());
}
