use nutype::nutype;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct InjectableScriptTree {
    scripts: Vec<ScriptEntry>,
    children: HashMap<String, InjectableScriptTree>,
}

#[derive(Debug, Deserialize)]
struct ScriptManifest {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    sites: HashMap<String, SiteNode>,
}

#[derive(Debug, Deserialize)]
struct SiteNode {
    scripts: Vec<ScriptEntry>,
    children: HashMap<String, SiteNode>,
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
    #[serde(rename = "requiresPro")]
    pub requires_pro: bool,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub content: String,
    #[serde(
        rename = "defaultEnabled",
        alias = "enabled",
        default = "default_enabled"
    )]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl ScriptEntry {
    fn allowed_for(&self, is_pro: bool) -> bool {
        self.enabled && (is_pro || !self.requires_pro)
    }
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

        if manifest.schema_version != 2 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Resistance catalog requires schemaVersion 2",
            ));
        }

        let mut tree = Self::default();
        let base_dir = json_path.parent().unwrap_or(Path::new("."));

        for (key, node) in manifest.sites {
            if key == "generic" {
                let generic_tree = Self::from_site_node(node, base_dir, "");
                tree.scripts.extend(generic_tree.scripts);
            } else {
                let child_tree = Self::from_site_node(node, base_dir, &key);
                tree.children.insert(key, child_tree);
            }
        }

        Ok(tree)
    }

    fn from_site_node(node: SiteNode, base_dir: &Path, domain: &str) -> Self {
        let mut tree = Self::default();

        for mut script_entry in node.scripts {
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
            let child_tree = Self::from_site_node(child_node, base_dir, &full_domain);
            tree.children.insert(child_name, child_tree);
        }

        tree
    }
    /// The host is parsed in reverse domain notation order (error.g. "sub.example.com" -> "com", "example", "sub").
    pub fn insert(&mut self, host: &str, script: String) {
        let entry = ScriptEntry {
            path: "".to_string(),
            name: "Manual Script".to_string(),
            description: "Inserted manually".to_string(),
            version: "1.0".to_string(),
            domain: "".to_string(),
            content: script,
            id: ScriptId::try_new(uuid::Uuid::now_v7().to_string())
                .expect("generated UUID is a nonempty script identifier"),
            enabled: true,
            requires_pro: false,
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

    pub fn has_scripts_of(&self, host: &str, is_pro: bool) -> bool {
        if self.scripts.iter().any(|entry| entry.allowed_for(is_pro)) {
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
                if node.scripts.iter().any(|entry| entry.allowed_for(is_pro)) {
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
    pub fn get_scripts(&self, host: &str, is_pro: bool) -> Vec<String> {
        let mut scripts = Vec::new();
        // Include root-level scripts (effectively global)
        scripts.extend(
            self.scripts
                .iter()
                .filter(|script| script.allowed_for(is_pro))
                .map(|script| script.content.clone()),
        );

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
                scripts.extend(
                    node.scripts
                        .iter()
                        .filter(|script| script.allowed_for(is_pro))
                        .map(|script| script.content.clone()),
                );
                current_node = node;
            } else {
                break;
            }
        }
        scripts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intervention_defaults_are_read_from_the_catalog() {
        let entry: ScriptEntry = serde_json::from_value(serde_json::json!({
            "id": "youtube_random_captcha", "path": "com/youtube/captcha.js",
            "name": "Captcha", "description": "An opt-in interruption",
            "version": "1.0.0", "defaultEnabled": false, "requiresPro": true
        }))
        .unwrap();
        assert!(!entry.enabled, "opt-in actions must not silently activate");
        let regular: ScriptEntry = serde_json::from_value(serde_json::json!({
            "id": "youtube_remove_shorts", "path": "com/youtube/shorts.js",
            "name": "Shorts", "description": "Remove shorts", "version": "1.0.0", "requiresPro": false
        }))
        .unwrap();
        assert!(regular.enabled, "non-disruptive scripts keep their default");
    }

    #[test]
    fn schema_two_and_explicit_tiers_are_required() {
        let directory = tempfile::tempdir().unwrap();
        let catalog = directory.path().join("scripts-data.json");
        fs::write(&catalog, r#"{"schemaVersion":1,"sites":{}}"#).unwrap();
        assert_eq!(
            InjectableScriptTree::load_from_json(&catalog)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidData
        );

        let script = serde_json::json!({
            "id": "example", "path": "example.js", "name": "Example",
            "description": "Example script", "version": "1.0",
            "defaultEnabled": true
        });
        assert!(
            serde_json::from_value::<ScriptEntry>(script).is_err(),
            "every Resistance entry must explicitly declare requiresPro"
        );
    }

    #[test]
    fn free_eligible_scripts_are_filtered_before_injection() {
        let mut tree = InjectableScriptTree::new();
        let mut free: ScriptEntry = serde_json::from_value(serde_json::json!({
            "id": "free", "path": "free.js", "name": "Free",
            "description": "Free", "version": "1.0", "requiresPro": false
        }))
        .unwrap();
        free.content = "free-script".to_string();
        let mut paid: ScriptEntry = serde_json::from_value(serde_json::json!({
            "id": "pro", "path": "pro.js", "name": "Pro",
            "description": "Pro", "version": "1.0", "requiresPro": true
        }))
        .unwrap();
        paid.content = "pro-script".to_string();
        tree.scripts = vec![free, paid];
        assert_eq!(tree.get_scripts("", false), vec!["free-script".to_string()]);
        assert_eq!(tree.get_scripts("", true).len(), 2);
        assert!(tree.has_scripts_of("", false));
        tree.scripts[0].enabled = false;
        assert!(!tree.has_scripts_of("", false));
        assert!(tree.has_scripts_of("", true));
    }

    #[test]
    fn test_script_inheritance() {
        let mut tree = InjectableScriptTree::new();

        tree.insert("com", "script_com".to_string());
        tree.insert("google.com", "script_google".to_string());
        tree.insert("mail.google.com", "script_mail".to_string());
        tree.insert("yahoo.com", "script_yahoo".to_string());

        // Test leaf node: should have all parents
        let scripts = tree.get_scripts("mail.google.com", false);
        assert!(scripts.contains(&"script_com".to_string()));
        assert!(scripts.contains(&"script_google".to_string()));
        assert!(scripts.contains(&"script_mail".to_string()));
        assert!(!scripts.contains(&"script_yahoo".to_string()));

        // Test intermediate node: should have parents but not children
        let scripts = tree.get_scripts("google.com", false);
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

        let scripts = tree.get_scripts("google.com", false);
        assert!(scripts.contains(&"com_script".to_string()));
        assert!(scripts.contains(&"google_script".to_string()));
        assert!(!scripts.contains(&"apple_script".to_string()));
    }

    #[test]
    fn test_unmatched_subdomain() {
        let mut tree = InjectableScriptTree::new();
        tree.insert("google.com", "google_script".to_string());

        // sub.google.com should inherit google.com
        let scripts = tree.get_scripts("sub.google.com", false);
        assert!(scripts.contains(&"google_script".to_string()));
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
