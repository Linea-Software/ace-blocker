use super::regex_rules::RegexRules;
use http::Uri;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt, str::FromStr};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserDefinedRuleType {
    Domain,
    Regex,
}

impl fmt::Display for UserDefinedRuleType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Domain => "domain",
            Self::Regex => "regex",
        })
    }
}

impl FromStr for UserDefinedRuleType {
    type Err = UserDefinedRulesError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "domain" => Ok(Self::Domain),
            "regex" => Ok(Self::Regex),
            other => Err(UserDefinedRulesError::InvalidRuleType(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserDefinedRule {
    pub rule: String,
    pub rule_type: UserDefinedRuleType,
}

impl UserDefinedRule {
    pub fn new(rule: String, rule_type: UserDefinedRuleType) -> Self {
        Self { rule, rule_type }
    }
}

#[derive(Error, Debug)]
pub enum UserDefinedRulesError {
    #[error("Invalid domain rule: {0}")]
    InvalidDomain(String),
    #[error("Invalid regex rule: {0}")]
    InvalidRegex(#[from] regex::Error),
    #[error("Invalid user rule type: {0}")]
    InvalidRuleType(String),
}

#[derive(Debug, Clone, Default)]
pub struct UserDefinedRules {
    rules: Vec<UserDefinedRule>,
    domains: HashSet<String>,
    regex_rules: RegexRules,
}

impl UserDefinedRules {
    pub fn new(rules: Vec<UserDefinedRule>) -> Result<Self, UserDefinedRulesError> {
        let normalized_rules = rules
            .into_iter()
            .map(Self::normalize_rule)
            .collect::<Result<Vec<_>, _>>()?;

        let domains = normalized_rules
            .iter()
            .filter(|rule| rule.rule_type == UserDefinedRuleType::Domain)
            .map(|rule| rule.rule.clone())
            .collect::<HashSet<_>>();

        let regex_patterns = normalized_rules
            .iter()
            .filter(|rule| rule.rule_type == UserDefinedRuleType::Regex)
            .map(|rule| rule.rule.clone())
            .collect::<Vec<_>>();

        Ok(Self {
            rules: normalized_rules,
            domains,
            regex_rules: RegexRules::new(regex_patterns)?,
        })
    }

    pub fn add_rules(
        &mut self,
        new_rules: Vec<UserDefinedRule>,
    ) -> Result<(), UserDefinedRulesError> {
        if new_rules.is_empty() {
            return Ok(());
        }

        let mut all_rules = self.rules.clone();
        all_rules.extend(new_rules);
        *self = Self::new(all_rules)?;
        Ok(())
    }

    pub fn remove_rule(&mut self, rule: &UserDefinedRule) -> Result<(), UserDefinedRulesError> {
        let normalized_rule = Self::normalize_rule(rule.clone())?;
        let mut all_rules = self.rules.clone();

        if let Some(position) = all_rules
            .iter()
            .position(|existing| existing == &normalized_rule)
        {
            all_rules.swap_remove(position);
            *self = Self::new(all_rules)?;
        }

        Ok(())
    }

    pub fn matches_uri(&self, uri: &Uri) -> bool {
        if let Some(host) = uri.host() {
            let normalized_host = normalize_host_for_match(host);
            if self.matches_domain(&normalized_host) {
                return true;
            }
        }

        self.regex_rules.contains(uri.to_string().as_str())
    }

    pub fn matches_host(&self, host: &str) -> bool {
        self.matches_domain(&normalize_host_for_match(host))
    }

    pub fn has_regex_rules(&self) -> bool {
        !self.regex_rules.is_empty()
    }

    pub fn rules(&self) -> &[UserDefinedRule] {
        &self.rules
    }

    fn normalize_rule(rule: UserDefinedRule) -> Result<UserDefinedRule, UserDefinedRulesError> {
        match rule.rule_type {
            UserDefinedRuleType::Domain => Ok(UserDefinedRule {
                rule: normalize_domain(&rule.rule)?,
                rule_type: UserDefinedRuleType::Domain,
            }),
            UserDefinedRuleType::Regex => {
                // Validate a regex before accepting it. The full RegexSet is rebuilt
                // below so matching stays consolidated and fast.
                regex::Regex::new(&rule.rule)?;
                Ok(rule)
            }
        }
    }

    fn matches_domain(&self, host: &str) -> bool {
        if self.domains.contains(host) {
            return true;
        }

        let mut suffix = host;
        while let Some(dot_index) = suffix.find('.') {
            suffix = &suffix[dot_index + 1..];
            if self.domains.contains(suffix) {
                return true;
            }
        }

        false
    }
}

fn normalize_host_for_match(host: &str) -> String {
    host.trim_end_matches('.').to_ascii_lowercase()
}

fn normalize_domain(domain: &str) -> Result<String, UserDefinedRulesError> {
    let normalized = domain.trim().trim_end_matches('.').to_ascii_lowercase();

    if normalized.is_empty() || normalized.len() > 253 {
        return Err(UserDefinedRulesError::InvalidDomain(domain.to_string()));
    }

    let labels = normalized.split('.').collect::<Vec<_>>();
    if labels.len() < 2 {
        return Err(UserDefinedRulesError::InvalidDomain(domain.to_string()));
    }

    for label in labels {
        let starts_alphanumeric = label
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric());
        let ends_alphanumeric = label
            .chars()
            .last()
            .is_some_and(|character| character.is_ascii_alphanumeric());

        if label.is_empty()
            || label.len() > 63
            || !starts_alphanumeric
            || !ends_alphanumeric
            || !label
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
        {
            return Err(UserDefinedRulesError::InvalidDomain(domain.to_string()));
        }
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn domain_rule(rule: &str) -> UserDefinedRule {
        UserDefinedRule::new(rule.to_string(), UserDefinedRuleType::Domain)
    }

    fn regex_rule(rule: &str) -> UserDefinedRule {
        UserDefinedRule::new(rule.to_string(), UserDefinedRuleType::Regex)
    }

    #[test]
    fn domain_rule_matches_domain_and_subdomains_only() {
        let rules = UserDefinedRules::new(vec![domain_rule("youtube.com")]).unwrap();

        for uri in [
            "https://youtube.com/",
            "https://www.youtube.com/watch?v=1",
            "https://a.b.youtube.com/path",
        ] {
            assert!(rules.matches_uri(&uri.parse::<Uri>().unwrap()));
        }

        for uri in [
            "https://hi-youtube.com/",
            "https://youtube.common/",
            "https://youtube.com.evil.com/",
            "https://example.com/?next=youtube.com",
        ] {
            assert!(!rules.matches_uri(&uri.parse::<Uri>().unwrap()));
        }
    }

    #[test]
    fn domain_rule_matching_is_case_insensitive_and_ignores_trailing_dot() {
        let rules = UserDefinedRules::new(vec![domain_rule("YouTube.COM.")]).unwrap();
        let uri = "https://WWW.YOUTUBE.COM./".parse::<Uri>().unwrap();

        assert!(rules.matches_uri(&uri));
        assert_eq!(rules.rules()[0].rule, "youtube.com");
    }

    #[test]
    fn invalid_domain_rule_is_rejected() {
        let result = UserDefinedRules::new(vec![domain_rule("https://youtube.com/path")]);
        assert!(matches!(
            result,
            Err(UserDefinedRulesError::InvalidDomain(_))
        ));
    }

    #[test]
    fn regex_rule_still_matches_full_uri() {
        let rules = UserDefinedRules::new(vec![regex_rule(r"/shorts(?:/|$)")]).unwrap();

        assert!(rules.matches_uri(&"https://youtube.com/shorts/abc".parse::<Uri>().unwrap()));
        assert!(!rules.matches_uri(&"https://youtube.com/watch?v=1".parse::<Uri>().unwrap()));
    }
}
