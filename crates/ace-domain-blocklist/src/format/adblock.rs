use crate::format::{
    DomainLineParsingError,
    DomainLineParsingResult::{self, Domains, Skip},
    DomainListFormat, DomainName, is_bare_ip,
};

pub struct AdblockFormat;

impl DomainListFormat for AdblockFormat {
    // parses a line with the following format:
    // ||<domain>^   (optionally followed by '$' options)
    fn parse_line(line: &str) -> Result<DomainLineParsingResult, DomainLineParsingError> {
        // First we're sanitizing the line
        let trimmed = line.trim();

        // Blank lines, comment lines ('!') and the Adblock Plus header marker.
        if trimmed.is_empty() || trimmed.starts_with('!') || trimmed.starts_with('[') {
            return Ok(Skip);
        }

        // Exception rules ('@@||domain^') explicitly tell NOT to block a
        // domain — they are ignored.
        if trimmed.starts_with("@@") {
            return Ok(Skip);
        }

        let Some(rule) = trimmed.strip_prefix("||") else {
            // Other Adblock constructs (cosmetic filters like
            // "example.com##.ad", regex rules, bare patterns) do not
            // represent blockable domains.
            return Ok(Skip);
        };

        // The rule may carry options after '^' or '$': "||domain^$important".
        let domain_part = rule.split(['^', '$']).next().unwrap_or_default().trim();

        if domain_part.is_empty() {
            return Err(DomainLineParsingError::InvalidLineFormat);
        }

        if is_bare_ip(domain_part) {
            return Err(DomainLineParsingError::InvalidDomainFormat {
                input: domain_part.to_string(),
            });
        }

        let domain = DomainName::try_new(domain_part).map_err(|_| {
            DomainLineParsingError::InvalidDomainFormat {
                input: domain_part.to_string(),
            }
        })?;

        Ok(Domains(vec![domain]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds the expected `Domains` result from the given domain literals.
    fn expected(domains: &[&str]) -> DomainLineParsingResult {
        Domains(
            domains
                .iter()
                .map(|domain| DomainName::try_new(*domain).unwrap())
                .collect(),
        )
    }

    // ── Skipped lines ───────────────────────────────────────────────

    #[test]
    fn blank_lines_are_skipped() {
        assert_eq!(AdblockFormat::parse_line(""), Ok(Skip));
        assert_eq!(AdblockFormat::parse_line("   "), Ok(Skip));
    }

    #[test]
    fn comment_lines_are_skipped() {
        assert_eq!(AdblockFormat::parse_line("!"), Ok(Skip));
        assert_eq!(
            AdblockFormat::parse_line("! Title: Light DNS Blocklist"),
            Ok(Skip)
        );
        assert_eq!(
            AdblockFormat::parse_line("! Version: 2026.0822.0800.26"),
            Ok(Skip)
        );
        assert_eq!(AdblockFormat::parse_line("   ! comment"), Ok(Skip));
    }

    #[test]
    fn abp_header_marker_is_skipped() {
        assert_eq!(AdblockFormat::parse_line("[Adblock Plus]"), Ok(Skip));
    }

    #[test]
    fn exception_rules_are_ignored() {
        // These lines tell the blocker NOT to block a domain.
        assert_eq!(AdblockFormat::parse_line("@@||example.com^"), Ok(Skip));
        assert_eq!(
            AdblockFormat::parse_line("@@||example.com^$important"),
            Ok(Skip)
        );
        assert_eq!(AdblockFormat::parse_line("@@||sub.example.com^"), Ok(Skip));
    }

    #[test]
    fn non_domain_rules_are_skipped() {
        // Cosmetic filters, regex rules, bare patterns and hosts lines carry
        // no domain that this importer could block.
        assert_eq!(
            AdblockFormat::parse_line("example.com##.ad-banner"),
            Ok(Skip)
        );
        assert_eq!(AdblockFormat::parse_line("/^https?://ads\\./"), Ok(Skip));
        assert_eq!(AdblockFormat::parse_line("adserver"), Ok(Skip));
        assert_eq!(AdblockFormat::parse_line("0.0.0.0 example.com"), Ok(Skip));
    }

    // ── Domain rules ────────────────────────────────────────────────

    #[test]
    fn domain_rules_are_parsed() {
        assert_eq!(
            AdblockFormat::parse_line("||example.com^"),
            Ok(expected(&["example.com"]))
        );
        assert_eq!(
            AdblockFormat::parse_line("||www.00fun.com^"),
            Ok(expected(&["www.00fun.com"]))
        );
        assert_eq!(
            AdblockFormat::parse_line("||sub.domain.example.com^"),
            Ok(expected(&["sub.domain.example.com"]))
        );
    }

    #[test]
    fn rules_without_caret_are_parsed() {
        assert_eq!(
            AdblockFormat::parse_line("||example.com"),
            Ok(expected(&["example.com"]))
        );
    }

    #[test]
    fn options_are_stripped() {
        assert_eq!(
            AdblockFormat::parse_line("||example.com^$important"),
            Ok(expected(&["example.com"]))
        );
        assert_eq!(
            AdblockFormat::parse_line("||example.com$third-party"),
            Ok(expected(&["example.com"]))
        );
    }

    #[test]
    fn valid_domain_shapes() {
        assert_eq!(
            AdblockFormat::parse_line("||xn--exmple-cua.com^"),
            Ok(expected(&["xn--exmple-cua.com"]))
        );
        assert_eq!(
            AdblockFormat::parse_line("||123.com^"),
            Ok(expected(&["123.com"]))
        );
    }

    #[test]
    fn underscore_domains_are_valid() {
        // Real-world lists contain underscore domains (e.g. "_domainkey").
        assert_eq!(
            AdblockFormat::parse_line("||_domainkey.youtube.com^"),
            Ok(expected(&["_domainkey.youtube.com"]))
        );
    }

    // ── Invalid lines ───────────────────────────────────────────────

    #[test]
    fn invalid_domains_are_rejected() {
        for invalid in ["bad..com", "-bad.com"] {
            let line = format!("||{invalid}^");
            match AdblockFormat::parse_line(&line) {
                Err(DomainLineParsingError::InvalidDomainFormat { input }) => {
                    assert_eq!(input, invalid);
                }
                other => panic!("expected InvalidDomainFormat for {invalid:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn bare_ip_rules_are_rejected() {
        match AdblockFormat::parse_line("||1.2.3.4^") {
            Err(DomainLineParsingError::InvalidDomainFormat { .. }) => {}
            other => panic!("expected InvalidDomainFormat, got {other:?}"),
        }
    }

    #[test]
    fn bare_pipe_marker_is_rejected() {
        match AdblockFormat::parse_line("||") {
            Err(DomainLineParsingError::InvalidLineFormat) => {}
            other => panic!("expected InvalidLineFormat, got {other:?}"),
        }
        match AdblockFormat::parse_line("||^") {
            Err(DomainLineParsingError::InvalidLineFormat) => {}
            other => panic!("expected InvalidLineFormat, got {other:?}"),
        }
    }
}
