use regex::{Error, RegexSet, RegexSetBuilder};

#[derive(Debug, Clone)]
pub struct RegexRules {
    set: RegexSet, // The Single Source of Truth
}

impl Default for RegexRules {
    fn default() -> Self {
        Self {
            set: RegexSet::empty(),
        }
    }
}

impl RegexRules {
    pub fn new(patterns: Vec<String>) -> Result<Self, Error> {
        let set = Self::build_set(&patterns)?;
        Ok(Self { set })
    }

    pub fn add_patterns(&mut self, new_patterns: Vec<String>) -> Result<(), Error> {
        if new_patterns.is_empty() {
            return Ok(());
        }

        let mut all_patterns = self.set.patterns().to_vec();

        all_patterns.extend(new_patterns);

        self.set = Self::build_set(&all_patterns)?;
        Ok(())
    }

    pub fn remove_pattern(&mut self, pattern: &str) -> Result<(), Error> {
        let mut all_patterns = self.set.patterns().to_vec();

        if let Some(pos) = all_patterns.iter().position(|x| x == pattern) {
            all_patterns.swap_remove(pos);

            self.set = Self::build_set(&all_patterns)?;
        }
        Ok(())
    }

    pub fn contains(&self, entry: &str) -> bool {
        self.set.is_match(entry)
    }

    pub fn patterns(&self) -> &[String] {
        self.set.patterns()
    }

    pub fn is_empty(&self) -> bool {
        self.set.patterns().is_empty()
    }

    fn build_set(patterns: &[String]) -> Result<RegexSet, Error> {
        RegexSetBuilder::new(patterns)
            .size_limit(100 * (1 << 20)) // 100 MB limit
            .dfa_size_limit(50 * (1 << 20)) // 50 MB cache limit
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_def_entries_basic_matching() {
        let patterns = vec![String::from("hello.com"), String::from("sample.net")];
        let user_def_entries = RegexRules::new(patterns).unwrap();

        assert!(user_def_entries.contains("hello.com"));
        assert!(user_def_entries.contains("hello.com/mypage"));
        assert!(user_def_entries.contains("sample.net/samplepage"));
        assert!(!user_def_entries.contains("sample"));
    }

    #[test]
    fn test_for_uri_path() {
        let patterns = vec![String::from("hello")];
        let user_def_entries = RegexRules::new(patterns).unwrap();

        assert!(user_def_entries.contains("hello"));
        assert!(user_def_entries.contains("mypage.com/hello"));
        assert!(user_def_entries.contains("mypage.com/hello2"));
    }

    #[test]
    fn test_add_patterns() {
        let mut entries = RegexRules::new(vec!["initial".to_string()]).unwrap();

        assert!(entries.contains("initial"));
        assert!(!entries.contains("added"));

        entries.add_patterns(vec!["added".to_string()]).unwrap();

        assert!(entries.contains("initial"));
        assert!(entries.contains("added"));
    }

    #[test]
    fn test_add_empty_pattern_list() {
        let mut entries = RegexRules::new(vec!["keep".to_string()]).unwrap();
        entries.add_patterns(vec![]).unwrap();

        assert!(entries.contains("keep"));
        assert_eq!(entries.patterns().len(), 1);
    }

    #[test]
    fn test_remove_pattern() {
        let mut entries = RegexRules::new(vec!["alpha".to_string(), "beta".to_string()]).unwrap();

        assert!(entries.contains("alpha"));
        assert!(entries.contains("beta"));

        entries.remove_pattern("alpha").unwrap();

        assert!(!entries.contains("alpha"));
        assert!(entries.contains("beta"));
    }

    #[test]
    fn test_remove_non_existent_pattern() {
        let mut entries = RegexRules::new(vec!["exist".to_string()]).unwrap();
        entries.remove_pattern("missing").unwrap();

        assert!(entries.contains("exist"));
        assert_eq!(entries.patterns().len(), 1);
    }

    #[test]
    fn test_default_implementation() {
        let mut entries = RegexRules::default();
        assert!(!entries.contains("anything"));

        entries.add_patterns(vec!["something".to_string()]).unwrap();
        assert!(entries.contains("something"));
    }

    #[test]
    fn test_complex_regex_anchors() {
        let patterns = vec![r"^\d{3}$".to_string()];
        let entries = RegexRules::new(patterns).unwrap();

        assert!(entries.contains("123"));
        assert!(!entries.contains("1234"));
        assert!(!entries.contains("a123"));
    }

    /// The seeded default-block rules (migration 0022) anchor their domains to
    /// host boundaries. Previously the seeds were bare domain strings matched
    /// with unanchored substring semantics, so `x.com` also matched
    /// `https://x.community/` and `https://notx.com/` — the over-match the
    /// founder's hybrid decision fixed. The engine tests rules against the full
    /// URI string, so the pattern requires a start / slash / dot before the
    /// host (bare host, scheme `://`, or subdomain) and a
    /// path/query/fragment/port separator or end after it.
    #[test]
    fn test_host_boundary_anchored_rules() {
        let patterns = vec![r"(?:^|[/.])x\.com(?:[/?#:]|$)".to_string()];
        let rules = RegexRules::new(patterns).unwrap();

        // The exact regressions from the over-match report:
        assert!(rules.contains("https://x.com/"));
        assert!(rules.contains("https://www.x.com/path"));
        assert!(!rules.contains("https://x.community/"));
        assert!(!rules.contains("https://notx.com/"));

        // Scheme, port, and bare-host variants of the same host still match.
        assert!(rules.contains("http://x.com/"));
        assert!(rules.contains("https://x.com:8443/path"));
        assert!(rules.contains("x.com"));

        // Hosts that merely contain `x.com` as a substring do not match.
        assert!(!rules.contains("https://x.com.evil.com/"));
        assert!(!rules.contains("https://evil.com/path?next=x.com"));
    }

    #[test]
    fn test_invalid_regex_error() {
        let patterns = vec![r"(".to_string()];
        let result = RegexRules::new(patterns);
        assert!(result.is_err());
    }

    #[test]
    fn test_pattern_list_retrieval() {
        let mut entries = RegexRules::new(vec!["a".to_string()]).unwrap();
        entries.add_patterns(vec!["b".to_string()]).unwrap();

        let patterns = entries.patterns();
        assert!(patterns.contains(&"a".to_string()));
        assert!(patterns.contains(&"b".to_string()));
        assert_eq!(patterns.len(), 2);
    }
}
