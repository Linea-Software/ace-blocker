use nutype::nutype;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path};

#[derive(Debug, Default, Clone)]
pub struct InjectableScriptTree {
    scripts: Vec<ScriptEntry>,
    children: HashMap<String, InjectableScriptTree>,
    /// Optional site-scoped dependency runtime injected before enabled scripts.
    shared_runtime: Option<String>,
    /// Catalog-relative shared chunks, cached once per loaded site node.
    shared_chunks: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct ScriptManifest {
    sites: HashMap<String, SiteNode>,
}

#[derive(Debug, Deserialize)]
struct SiteNode {
    scripts: Vec<ScriptEntry>,
    children: HashMap<String, SiteNode>,
    #[serde(rename = "runtimePath", default)]
    runtime_path: Option<String>,
}

#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Hash,
        Serialize,
        Deserialize,
        TryFrom,
        FromStr,
        Display,
        Deref,
        AsRef
    )
)]
pub struct ScriptId(String);

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ScriptEntry {
    pub id: ScriptId,
    pub path: String,
    pub name: String,
    pub description: String,
    pub version: String,
    #[serde(rename = "sharedChunks", default, skip_serializing)]
    pub shared_chunks: Vec<String>,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub content: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn read_shared_asset(base_dir: &Path, relative_path: &str) -> std::io::Result<String> {
    // The catalog originates from an imported ZIP. Never allow an executable
    // asset to refer outside its extraction directory.
    let path = Path::new(relative_path);
    if relative_path.contains(['\\', ':'])
        || !relative_path.ends_with(".js")
        || relative_path
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid resistance shared asset path",
        ));
    }
    // Reject symlink escapes as well as lexical traversal. The resolved file
    // must remain underneath the extracted catalog directory.
    let root = fs::canonicalize(base_dir)?;
    let asset = fs::canonicalize(base_dir.join(path))?;
    if !asset.starts_with(&root) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "shared asset resolves outside resistance catalog",
        ));
    }
    fs::read_to_string(asset)
}

fn default_enabled() -> bool {
    true
}

impl InjectableScriptTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_all_script_entries(&self) -> Vec<ScriptEntry> {
        let mut entries = Vec::new();
        self.get_all_script_entries_rec("", &mut entries);
        entries
    }

    pub fn get_script_by_id(&self, id: &ScriptId) -> Option<ScriptEntry> {
        for entry in &self.scripts {
            if &entry.id == id {
                return Some(entry.clone());
            }
        }
        for child in self.children.values() {
            if let Some(entry) = child.get_script_by_id(id) {
                return Some(entry);
            }
        }
        None
    }

    pub fn set_script_enabled(&mut self, id: &ScriptId, enabled: bool) -> bool {
        for entry in &mut self.scripts {
            if &entry.id == id {
                entry.enabled = enabled;
                return true;
            }
        }
        for child in self.children.values_mut() {
            if child.set_script_enabled(id, enabled) {
                return true;
            }
        }
        false
    }

    fn get_all_script_entries_rec(&self, domain_path: &str, entries: &mut Vec<ScriptEntry>) {
        for entry in &self.scripts {
            let mut entry = entry.clone();
            if entry.domain.is_empty() {
                entry.domain = if domain_path.is_empty() {
                    "global".to_string()
                } else {
                    domain_path.to_string()
                };
            }
            entries.push(entry);
        }
        for (name, child) in &self.children {
            let next_domain = if domain_path.is_empty() {
                name.clone()
            } else {
                format!("{}.{}", name, domain_path)
            };
            child.get_all_script_entries_rec(&next_domain, entries);
        }
    }

    pub fn load_from_json(json_path: &Path) -> std::io::Result<Self> {
        let content = fs::read_to_string(json_path)?;
        let manifest: ScriptManifest = serde_json::from_str(&content)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;

        let mut tree = Self::default();
        let base_dir = json_path.parent().unwrap_or(Path::new("."));

        for (key, node) in manifest.sites {
            if key == "generic" {
                let generic_tree = Self::from_site_node(node, base_dir, "")?;
                tree.scripts.extend(generic_tree.scripts);
                tree.shared_runtime = generic_tree.shared_runtime;
                tree.shared_chunks = generic_tree.shared_chunks;
            } else {
                let child_tree = Self::from_site_node(node, base_dir, &key)?;
                tree.children.insert(key, child_tree);
            }
        }

        Ok(tree)
    }

    fn from_site_node(node: SiteNode, base_dir: &Path, domain: &str) -> std::io::Result<Self> {
        let mut tree = Self::default();

        if let Some(runtime_path) = node.runtime_path.as_deref() {
            tree.shared_runtime = Some(read_shared_asset(base_dir, runtime_path)?);
        }

        for mut script_entry in node.scripts {
            if !script_entry.shared_chunks.is_empty() && tree.shared_runtime.is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "shared chunks require a site runtime",
                ));
            }
            for path in &script_entry.shared_chunks {
                if !tree.shared_chunks.contains_key(path) {
                    let source = read_shared_asset(base_dir, path)?;
                    tree.shared_chunks.insert(path.clone(), source);
                }
            }
            script_entry.domain = domain.to_string();
            let script_path = base_dir.join(&script_entry.path);

            if let Ok(script_content) = fs::read_to_string(&script_path) {
                script_entry.content = script_content;
                tree.scripts.push(script_entry);
            } else {
                // TODO: ignore or else?
            }
        }

        for (child_name, child_node) in node.children {
            let full_domain = format!("{}.{}", child_name, domain);
            let child_tree = Self::from_site_node(child_node, base_dir, &full_domain)?;
            tree.children.insert(child_name, child_tree);
        }

        Ok(tree)
    }
    /// The host is parsed in reverse domain notation order (error.g. "sub.example.com" -> "com", "example", "sub").
    pub fn insert(&mut self, host: &str, script: String) {
        let entry = ScriptEntry {
            path: "".to_string(),
            name: "Manual Script".to_string(),
            description: "Inserted manually".to_string(),
            version: "1.0".to_string(),
            shared_chunks: Vec::new(),
            domain: "".to_string(),
            content: script,
            id: ScriptId::try_new(uuid::Uuid::now_v7().to_string())
                .expect("generated UUID is a nonempty script identifier"),
            enabled: true,
        };

        if host.is_empty() {
            self.scripts.push(entry);
            return;
        }

        let parts: Vec<&str> = host.split('.').rev().collect();
        let mut current_node = self;

        for part in parts {
            current_node = current_node.children.entry(part.to_string()).or_default();
        }
        current_node.scripts.push(entry);
    }

    pub fn has_scripts_of(&self, host: &str) -> bool {
        if self.scripts.iter().any(|error| error.enabled) {
            return true;
        }

        if host.is_empty() {
            return false;
        }

        let host_clean = host.split(':').next().unwrap_or(host).to_lowercase();
        let host_clean = if let Some(stripped) = host_clean.strip_prefix("www.") {
            stripped
        } else {
            &host_clean
        };

        let parts: Vec<&str> = host_clean.split('.').rev().collect();
        let mut current_node = self;

        for part in parts {
            if let Some(node) = current_node.children.get(part) {
                if node.scripts.iter().any(|error| error.enabled) {
                    return true;
                }
                current_node = node;
            } else {
                break;
            }
        }
        false
    }

    /// Get all scripts to apply to a certain host name.
    /// Scripts are collected from the root down to the specific host node (parents included).
    /// Children of the matched node are NOT included.
    pub fn get_scripts(&self, host: &str) -> Vec<String> {
        let mut scripts = Vec::new();
        // Include root-level scripts (effectively global)
        self.append_enabled_scripts(&mut scripts);

        if host.is_empty() {
            return scripts;
        }
        // Clean host: remove port and lowercase
        let host_clean = host.split(':').next().unwrap_or(host).to_lowercase();
        let host_clean = if let Some(stripped) = host_clean.strip_prefix("www.") {
            stripped
        } else {
            &host_clean
        };

        let parts: Vec<&str> = host_clean.split('.').rev().collect();

        let mut current_node = self;

        for part in parts {
            if let Some(node) = current_node.children.get(part) {
                node.append_enabled_scripts(&mut scripts);
                current_node = node;
            } else {
                break;
            }
        }
        scripts
    }

    fn append_enabled_scripts(&self, scripts: &mut Vec<String>) {
        if self.scripts.iter().any(|script| script.enabled) {
            if let Some(runtime) = &self.shared_runtime {
                scripts.push(runtime.clone());
                let mut injected = HashSet::new();
                for script in self.scripts.iter().filter(|script| script.enabled) {
                    for path in &script.shared_chunks {
                        if injected.insert(path) {
                            // Validated and loaded when the catalog was imported.
                            if let Some(source) = self.shared_chunks.get(path) {
                                scripts.push(source.clone());
                            }
                        }
                    }
                }
            }
            scripts.extend(
                self.scripts
                    .iter()
                    .filter(|script| script.enabled)
                    .map(|script| script.content.clone()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_inheritance() {
        let mut tree = InjectableScriptTree::new();

        tree.insert("com", "script_com".to_string());
        tree.insert("google.com", "script_google".to_string());
        tree.insert("mail.google.com", "script_mail".to_string());
        tree.insert("yahoo.com", "script_yahoo".to_string());

        // Test leaf node: should have all parents
        let scripts = tree.get_scripts("mail.google.com");
        assert!(scripts.contains(&"script_com".to_string()));
        assert!(scripts.contains(&"script_google".to_string()));
        assert!(scripts.contains(&"script_mail".to_string()));
        assert!(!scripts.contains(&"script_yahoo".to_string()));

        // Test intermediate node: should have parents but not children
        let scripts = tree.get_scripts("google.com");
        assert!(scripts.contains(&"script_com".to_string()));
        assert!(scripts.contains(&"script_google".to_string()));
        assert!(!scripts.contains(&"script_mail".to_string()));
    }

    #[test]
    fn test_exact_match_priority() {
        // If I have com.google and I request com.google, I get parent (com) and self (google).
        // I should not get other children of com.
        let mut tree = InjectableScriptTree::new();
        tree.insert("com", "com_script".to_string());
        tree.insert("google.com", "google_script".to_string());
        tree.insert("apple.com", "apple_script".to_string());

        let scripts = tree.get_scripts("google.com");
        assert!(scripts.contains(&"com_script".to_string()));
        assert!(scripts.contains(&"google_script".to_string()));
        assert!(!scripts.contains(&"apple_script".to_string()));
    }

    #[test]
    fn test_unmatched_subdomain() {
        let mut tree = InjectableScriptTree::new();
        tree.insert("google.com", "google_script".to_string());

        // sub.google.com should inherit google.com
        let scripts = tree.get_scripts("sub.google.com");
        assert!(scripts.contains(&"google_script".to_string()));
    }
}

#[cfg(test)]
mod shared_runtime_tests {
    use super::*;
    use serde_json::json;

    fn write_catalog(directory: &Path, runtime_path: &str) -> std::path::PathBuf {
        let catalog_path = directory.join("scripts-data.json");
        let catalog = json!({
            "sites": {
                "com": {
                    "scripts": [],
                    "children": {
                        "example": {
                            "scripts": [{
                                "id": "example-test",
                                "path": "example.js",
                                "name": "Example",
                                "description": "Example script",
                                "version": "1"
                            }],
                            "children": {},
                            "runtimePath": runtime_path
                        }
                    }
                }
            }
        });
        fs::write(&catalog_path, catalog.to_string()).unwrap();
        catalog_path
    }

    #[test]
    fn runtime_precedes_scripts_and_is_skipped_when_all_are_disabled() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("runtime.js"), "shared runtime;").unwrap();
        fs::write(directory.path().join("example.js"), "site entry;").unwrap();
        let catalog = write_catalog(directory.path(), "runtime.js");
        let mut tree = InjectableScriptTree::load_from_json(&catalog).unwrap();
        assert_eq!(
            tree.get_scripts("www.example.com"),
            vec!["shared runtime;", "site entry;"]
        );
        assert!(tree.set_script_enabled(&ScriptId::try_new("example-test").unwrap(), false));
        assert!(tree.get_scripts("example.com").is_empty());
        assert!(tree.get_scripts("other.com").is_empty());
    }

    #[test]
    fn only_chunks_of_enabled_scripts_are_injected_and_deduplicated() {
        let directory = tempfile::tempdir().unwrap();
        for (name, content) in [
            ("runtime.js", "bootstrap;"),
            ("shared.js", "register shared;"),
            ("extra.js", "register extra;"),
            ("first.js", "first entry;"),
            ("second.js", "second entry;"),
        ] {
            fs::write(directory.path().join(name), content).unwrap();
        }
        let catalog_path = directory.path().join("scripts-data.json");
        let catalog = json!({
            "sites": {
                "com": {
                    "scripts": [],
                    "children": {
                        "example": {
                            "runtimePath": "runtime.js",
                            "scripts": [
                                {
                                    "id": "first",
                                    "path": "first.js",
                                    "name": "First",
                                    "description": "First",
                                    "version": "1",
                                    "sharedChunks": ["shared.js"]
                                },
                                {
                                    "id": "second",
                                    "path": "second.js",
                                    "name": "Second",
                                    "description": "Second",
                                    "version": "1",
                                    "sharedChunks": ["shared.js", "extra.js"]
                                }
                            ],
                            "children": {}
                        }
                    }
                }
            }
        });
        fs::write(&catalog_path, catalog.to_string()).unwrap();
        let mut tree = InjectableScriptTree::load_from_json(&catalog_path).unwrap();
        assert_eq!(
            tree.get_scripts("example.com"),
            vec![
                "bootstrap;",
                "register shared;",
                "register extra;",
                "first entry;",
                "second entry;"
            ]
        );
        tree.set_script_enabled(&ScriptId::try_new("second").unwrap(), false);
        assert_eq!(
            tree.get_scripts("example.com"),
            vec!["bootstrap;", "register shared;", "first entry;"]
        );
        tree.set_script_enabled(&ScriptId::try_new("first").unwrap(), false);
        assert!(tree.get_scripts("example.com").is_empty());

        // A missing declared chunk makes the pack invalid at load time.
        fs::remove_file(directory.path().join("shared.js")).unwrap();
        assert_eq!(
            InjectableScriptTree::load_from_json(&catalog_path)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::NotFound
        );
    }

    #[cfg(unix)]
    #[test]
    fn shared_runtime_symlink_cannot_escape_the_import_directory() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_runtime = outside.path().join("outside.js");
        fs::write(&outside_runtime, "outside;").unwrap();
        symlink(&outside_runtime, directory.path().join("runtime.js")).unwrap();
        fs::write(directory.path().join("example.js"), "entry;").unwrap();
        let catalog = write_catalog(directory.path(), "runtime.js");
        assert_eq!(
            InjectableScriptTree::load_from_json(&catalog)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn invalid_or_missing_runtime_fails_catalog_load() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("example.js"), "site entry;").unwrap();
        for runtime_path in ["../escape.js", "/absolute.js", "C:escape.js", "a\\b.js"] {
            let catalog = write_catalog(directory.path(), runtime_path);
            assert_eq!(
                InjectableScriptTree::load_from_json(&catalog)
                    .unwrap_err()
                    .kind(),
                std::io::ErrorKind::InvalidData
            );
        }
        let catalog = write_catalog(directory.path(), "missing.js");
        assert_eq!(
            InjectableScriptTree::load_from_json(&catalog)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::NotFound
        );
    }
}

#[cfg(test)]
mod id_validation_tests {
    use super::ScriptId;

    #[test]
    fn script_id_rejects_empty_and_whitespace_values() {
        assert!(ScriptId::try_new("").is_err());
        assert!(ScriptId::try_new("   ").is_err());
        assert!(serde_json::from_str::<ScriptId>(r#""""#).is_err());
    }
}
