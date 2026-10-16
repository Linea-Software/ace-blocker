use crate::format::{
    DomainLineParsingError,
    DomainLineParsingResult::{self, Domains, Skip},
    DomainListFormat, DomainName, is_bare_ip,
};
use std::net::IpAddr;

pub struct HostsListStandardFormat;

impl DomainListFormat for HostsListStandardFormat {
    // parses a line with the following format:
    // <redirect> <hostname> <aliases[]>
    fn parse_line(line: &str) -> Result<DomainLineParsingResult, DomainLineParsingError> {
        // First we're sanitizing the line
        let trimmed = line.trim();

        let line = match trimmed.split_once('#') {
            // Split has happened
            Some((uncommented_line, _)) => uncommented_line,
            // no comment in the line
            None => trimmed,
        };

        if line.is_empty() {
            return Ok(Skip);
        }

        let parts: Vec<&str> = line.split_whitespace().collect();

        // A hosts line requires a redirect IP and at least one hostname.
        if parts.len() < 2 {
            return Err(DomainLineParsingError::InvalidLineFormat);
        }

        if !is_valid_redirect(parts[0]) {
            return Err(DomainLineParsingError::InvalidRedirectFormat {
                input: parts[0].to_string(),
            });
        }

        let mut domains: Vec<DomainName> = Vec::new();

        for part in &parts[1..] {
            if is_bare_ip(part) {
                return Err(DomainLineParsingError::InvalidDomainFormat {
                    input: part.to_string(),
                });
            }

            let domain = DomainName::try_new(*part).map_err(|_| {
                DomainLineParsingError::InvalidDomainFormat {
                    input: part.to_string(),
                }
            })?;
            domains.push(domain);
        }
        Ok(Domains(domains))
    }
}

/// Returns true when the token is a valid redirect IP.
///
/// IPv6 zone identifiers (`fe80::1%lo0`) appear in macOS hosts files and are
/// stripped before parsing.
fn is_valid_redirect(token: &str) -> bool {
    token
        .split('%')
        .next()
        .unwrap_or(token)
        .parse::<IpAddr>()
        .is_ok()
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

    // ── Blank lines ─────────────────────────────────────────────────

    #[test]
    fn blank_lines_are_skipped() {
        assert_eq!(HostsListStandardFormat::parse_line(""), Ok(Skip));
        assert_eq!(HostsListStandardFormat::parse_line("   "), Ok(Skip));
        assert_eq!(HostsListStandardFormat::parse_line("\t"), Ok(Skip));
        assert_eq!(HostsListStandardFormat::parse_line("  \t  "), Ok(Skip));
    }

    // ── Comments ───────────────────────────────────────────────────

    #[test]
    fn full_line_comments_are_skipped() {
        assert_eq!(HostsListStandardFormat::parse_line("#"), Ok(Skip));
        assert_eq!(HostsListStandardFormat::parse_line("# comment"), Ok(Skip));
        assert_eq!(HostsListStandardFormat::parse_line("  # comment"), Ok(Skip));
        assert_eq!(HostsListStandardFormat::parse_line("\t# comment"), Ok(Skip));
        assert_eq!(HostsListStandardFormat::parse_line("##"), Ok(Skip));
        assert_eq!(HostsListStandardFormat::parse_line("# !!! ! !"), Ok(Skip));
    }

    #[test]
    fn trailing_comments_are_stripped() {
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0 example.com # comment"),
            Ok(expected(&["example.com"]))
        );
        // No space needed before the comment marker.
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0 example.com#comment"),
            Ok(expected(&["example.com"]))
        );
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0 example.com#"),
            Ok(expected(&["example.com"]))
        );
        // A tab before the comment marker is fine too.
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0 example.com\t# comment"),
            Ok(expected(&["example.com"]))
        );
        // Only the first '#' starts the comment.
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0 example.com # see # here"),
            Ok(expected(&["example.com"]))
        );
        // Aliases before the comment survive.
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0 example.com www.example.com # comment"),
            Ok(expected(&["example.com", "www.example.com"]))
        );
    }

    // ── Basic structure ────────────────────────────────────────────

    #[test]
    fn single_host_lines() {
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0 example.com"),
            Ok(expected(&["example.com"]))
        );
        assert_eq!(
            HostsListStandardFormat::parse_line("127.0.0.1 localhost"),
            Ok(expected(&["localhost"]))
        );
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0 a.com"),
            Ok(expected(&["a.com"]))
        );
    }

    #[test]
    fn multiple_aliases_are_parsed() {
        assert_eq!(
            HostsListStandardFormat::parse_line(
                "0.0.0.0 example.com www.example.com api.example.com"
            ),
            Ok(expected(&[
                "example.com",
                "www.example.com",
                "api.example.com"
            ]))
        );
    }

    #[test]
    fn aliases_keep_their_order() {
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0 z.com a.com m.com"),
            Ok(expected(&["z.com", "a.com", "m.com"]))
        );
    }

    #[test]
    fn redirect_ip_variants_are_accepted() {
        for redirect in [
            "0.0.0.0",
            "127.0.0.1",
            "255.255.255.255",
            "::1",
            "fe80::1",
            "::",
        ] {
            let line = format!("{redirect} example.com");
            assert_eq!(
                HostsListStandardFormat::parse_line(&line),
                Ok(expected(&["example.com"])),
                "redirect: {redirect}"
            );
        }
    }

    // ── Whitespace handling ────────────────────────────────────────

    #[test]
    fn tabs_and_multiple_spaces_are_separators() {
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0\texample.com"),
            Ok(expected(&["example.com"]))
        );
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0\t\texample.com\twww.example.com"),
            Ok(expected(&["example.com", "www.example.com"]))
        );
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0   example.com    www.example.com"),
            Ok(expected(&["example.com", "www.example.com"]))
        );
    }

    #[test]
    fn leading_and_trailing_whitespace_is_trimmed() {
        assert_eq!(
            HostsListStandardFormat::parse_line("   0.0.0.0 example.com   "),
            Ok(expected(&["example.com"]))
        );
        assert_eq!(
            HostsListStandardFormat::parse_line("\t0.0.0.0 example.com\t"),
            Ok(expected(&["example.com"]))
        );
    }

    #[test]
    fn trailing_carriage_return_is_trimmed() {
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0 example.com\r"),
            Ok(expected(&["example.com"]))
        );
    }

    // ── Valid domain shapes ─────────────────────────────────────────

    #[test]
    fn deep_subdomains_are_valid() {
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0 a.b.c.d.example.com"),
            Ok(expected(&["a.b.c.d.example.com"]))
        );
    }

    #[test]
    fn numeric_labels_are_valid() {
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0 123.com"),
            Ok(expected(&["123.com"]))
        );
    }

    #[test]
    fn punycode_idns_are_valid() {
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0 xn--exmple-cua.com"),
            Ok(expected(&["xn--exmple-cua.com"]))
        );
    }

    #[test]
    fn underscore_domains_are_valid() {
        // Real-world lists contain underscore domains (e.g. "_domainkey").
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0 _domainkey.example.com"),
            Ok(expected(&["_domainkey.example.com"]))
        );
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0 forgotten_dark_angel.tripod.com"),
            Ok(expected(&["forgotten_dark_angel.tripod.com"]))
        );
    }

    #[test]
    fn ipv6_zone_id_redirects_are_accepted() {
        // macOS hosts files use link-local addresses with zone identifiers.
        assert_eq!(
            HostsListStandardFormat::parse_line("fe80::1%lo0 localhost"),
            Ok(expected(&["localhost"]))
        );
    }

    #[test]
    fn max_length_label_is_valid() {
        let label = "a".repeat(63);
        let domain = format!("{label}.com");
        let line = format!("0.0.0.0 {domain}");
        assert_eq!(
            HostsListStandardFormat::parse_line(&line),
            Ok(expected(&[domain.as_str()]))
        );
    }

    #[test]
    fn uppercase_hostnames_are_preserved() {
        assert_eq!(
            HostsListStandardFormat::parse_line("0.0.0.0 Example.COM"),
            Ok(expected(&["Example.COM"]))
        );
    }

    // ── Invalid domains ────────────────────────────────────────────

    #[test]
    fn invalid_domains_are_rejected() {
        for invalid in [
            "bad..com",    // empty label
            "-bad.com",    // label starts with a hyphen
            "bad-.com",    // label ends with a hyphen
            "exämple.com", // non-ASCII: IDN must be punycode
        ] {
            let line = format!("0.0.0.0 {invalid}");
            match HostsListStandardFormat::parse_line(&line) {
                Err(DomainLineParsingError::InvalidDomainFormat { input }) => {
                    assert_eq!(input, invalid);
                }
                other => panic!("expected InvalidDomainFormat for {invalid:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn label_longer_than_63_chars_is_rejected() {
        let label = "a".repeat(64);
        let line = format!("0.0.0.0 {label}.com");
        match HostsListStandardFormat::parse_line(&line) {
            Err(DomainLineParsingError::InvalidDomainFormat { .. }) => {}
            other => panic!("expected InvalidDomainFormat, got {other:?}"),
        }
    }

    #[test]
    fn error_identifies_the_offending_token() {
        // Parsing aborts at the first invalid token.
        match HostsListStandardFormat::parse_line("0.0.0.0 good.com bad..com") {
            Err(DomainLineParsingError::InvalidDomainFormat { input }) => {
                assert_eq!(input, "bad..com");
            }
            other => panic!("expected InvalidDomainFormat, got {other:?}"),
        }
    }

    // ── Format strictness ───────────────────────────────────────────
    // The documented format is `<redirect> <hostname> <aliases[]>`; the
    // following tests assert that lines deviating from it are rejected.
    // NOTE: some of these currently fail — the parser does not validate
    // the redirect token nor require at least one hostname.

    #[test]
    fn line_without_redirect_is_rejected() {
        // A bare domain is wildcard-format, not hosts-format.
        match HostsListStandardFormat::parse_line("example.com") {
            Err(DomainLineParsingError::InvalidLineFormat) => {}
            other => panic!("expected InvalidLineFormat, got {other:?}"),
        }
    }

    #[test]
    fn line_with_redirect_but_no_hostname_is_rejected() {
        match HostsListStandardFormat::parse_line("0.0.0.0") {
            Err(DomainLineParsingError::InvalidLineFormat) => {}
            other => panic!("expected InvalidLineFormat, got {other:?}"),
        }
    }

    #[test]
    fn line_where_comment_eats_the_hostname_is_rejected() {
        match HostsListStandardFormat::parse_line("0.0.0.0 # comment") {
            Err(DomainLineParsingError::InvalidLineFormat) => {}
            other => panic!("expected InvalidLineFormat, got {other:?}"),
        }
    }

    #[test]
    fn non_ip_redirect_is_rejected() {
        match HostsListStandardFormat::parse_line("example.com www.example.com") {
            Err(DomainLineParsingError::InvalidRedirectFormat { .. }) => {}
            other => panic!("expected InvalidRedirectFormat, got {other:?}"),
        }
    }

    #[test]
    fn ip_address_as_hostname_is_rejected() {
        // A blocklist entry is a hostname, not an address literal.
        match HostsListStandardFormat::parse_line("0.0.0.0 1.2.3.4") {
            Err(DomainLineParsingError::InvalidDomainFormat { .. }) => {}
            other => panic!("expected InvalidDomainFormat, got {other:?}"),
        }
    }
}
