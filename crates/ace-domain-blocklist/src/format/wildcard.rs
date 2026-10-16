use crate::format::{
    DomainLineParsingError,
    DomainLineParsingResult::{self, Domains, Skip},
    DomainListFormat, DomainName,
};

pub struct WildcardDomainsFormat;

impl DomainListFormat for WildcardDomainsFormat {
    // parses a line with the following format:
    // <domain>
    fn parse_line(line: &str) -> Result<DomainLineParsingResult, DomainLineParsingError> {
        // First we're sanitizing the line
        let trimmed = line.trim();

        // Comments start with '#' (e.g. the hagezi header block)
        let line = trimmed
            .split_once('#')
            .map_or(trimmed, |(uncommented_line, _)| uncommented_line)
            .trim();

        if line.is_empty() {
            return Ok(Skip);
        }

        // A single token is expected: whitespace means the line belongs to
        // another format (e.g. "0.0.0.0 example.com" is hosts-format).
        if line.contains(char::is_whitespace) {
            return Err(DomainLineParsingError::InvalidLineFormat);
        }

        // Bare IP entries are accepted: real-world lists (e.g. ut1) contain
        // raw IP addresses to block.

        let domain =
            DomainName::try_new(line).map_err(|_| DomainLineParsingError::InvalidDomainFormat {
                input: line.to_string(),
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

    // ── Blank lines and comments ────────────────────────────────────

    #[test]
    fn blank_lines_are_skipped() {
        assert_eq!(WildcardDomainsFormat::parse_line(""), Ok(Skip));
        assert_eq!(WildcardDomainsFormat::parse_line("   "), Ok(Skip));
        assert_eq!(WildcardDomainsFormat::parse_line("\t"), Ok(Skip));
    }

    #[test]
    fn comment_lines_are_skipped() {
        assert_eq!(WildcardDomainsFormat::parse_line("#"), Ok(Skip));
        assert_eq!(
            WildcardDomainsFormat::parse_line("# Title: Light DNS Blocklist"),
            Ok(Skip)
        );
        assert_eq!(
            WildcardDomainsFormat::parse_line("# Number of entries: 42350"),
            Ok(Skip)
        );
        assert_eq!(WildcardDomainsFormat::parse_line("   # comment"), Ok(Skip));
    }

    #[test]
    fn trailing_comments_are_stripped() {
        assert_eq!(
            WildcardDomainsFormat::parse_line("example.com # comment"),
            Ok(expected(&["example.com"]))
        );
        assert_eq!(
            WildcardDomainsFormat::parse_line("example.com#comment"),
            Ok(expected(&["example.com"]))
        );
        assert_eq!(
            WildcardDomainsFormat::parse_line("example.com\t# comment"),
            Ok(expected(&["example.com"]))
        );
    }

    // ── Valid domains ───────────────────────────────────────────────

    #[test]
    fn single_domains_are_parsed() {
        assert_eq!(
            WildcardDomainsFormat::parse_line("example.com"),
            Ok(expected(&["example.com"]))
        );
        assert_eq!(
            WildcardDomainsFormat::parse_line("ads.example.com"),
            Ok(expected(&["ads.example.com"]))
        );
        assert_eq!(
            WildcardDomainsFormat::parse_line("www.00fun.com"),
            Ok(expected(&["www.00fun.com"]))
        );
    }

    #[test]
    fn valid_domain_shapes() {
        assert_eq!(
            WildcardDomainsFormat::parse_line("xn--exmple-cua.com"),
            Ok(expected(&["xn--exmple-cua.com"]))
        );
        assert_eq!(
            WildcardDomainsFormat::parse_line("123.com"),
            Ok(expected(&["123.com"]))
        );
    }

    #[test]
    fn underscore_domains_are_valid() {
        // Real-world lists contain underscore domains (e.g. "_domainkey").
        assert_eq!(
            WildcardDomainsFormat::parse_line("_domainkey.youtube.com"),
            Ok(expected(&["_domainkey.youtube.com"]))
        );
    }

    // ── Invalid lines ───────────────────────────────────────────────

    #[test]
    fn hosts_format_lines_are_rejected() {
        // "0.0.0.0 example.com" belongs to the hosts format, not wildcard.
        match WildcardDomainsFormat::parse_line("0.0.0.0 example.com") {
            Err(DomainLineParsingError::InvalidLineFormat) => {}
            other => panic!("expected InvalidLineFormat, got {other:?}"),
        }
    }

    #[test]
    fn adblock_format_lines_are_rejected() {
        // "||example.com^" belongs to the adblock format, not wildcard.
        match WildcardDomainsFormat::parse_line("||example.com^") {
            Err(DomainLineParsingError::InvalidDomainFormat { .. }) => {}
            other => panic!("expected InvalidDomainFormat, got {other:?}"),
        }
    }

    #[test]
    fn bare_ip_entries_are_accepted() {
        // Real-world lists (e.g. ut1) contain raw IP entries to block.
        assert_eq!(
            WildcardDomainsFormat::parse_line("1.0.0.4"),
            Ok(expected(&["1.0.0.4"]))
        );
        assert_eq!(
            WildcardDomainsFormat::parse_line("206.132.42.196"),
            Ok(expected(&["206.132.42.196"]))
        );
    }

    #[test]
    fn invalid_domains_are_rejected() {
        for invalid in ["bad..com", "-bad.com", "bad-.com"] {
            match WildcardDomainsFormat::parse_line(invalid) {
                Err(DomainLineParsingError::InvalidDomainFormat { input }) => {
                    assert_eq!(input, invalid);
                }
                other => panic!("expected InvalidDomainFormat for {invalid:?}, got {other:?}"),
            }
        }
    }
}
