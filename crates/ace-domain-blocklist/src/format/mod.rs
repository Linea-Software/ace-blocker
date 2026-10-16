pub mod adblock;
pub mod hosts;
pub mod wildcard;

pub use adblock::AdblockFormat;
pub use hosts::HostsListStandardFormat;
pub use wildcard::WildcardDomainsFormat;

use nutype::nutype;
use std::net::IpAddr;
use thiserror::Error;

#[nutype(
    validate(predicate = is_valid_domain),
    derive(Debug, PartialEq)
)]
pub struct DomainName(String);

/// Validates a domain name as found in blocklists.
///
/// DNS hostnames (RFC 1035/1123) technically only allow letters, digits and
/// hyphens, but real-world blocklists contain underscore domains (e.g.
/// `_domainkey.example.com`, legacy webhosts). Since entries are matched by
/// hostname, underscores are accepted here.
fn is_valid_domain(input: &str) -> bool {
    if input.is_empty() || input.len() > 253 {
        return false;
    }
    if input.starts_with('.') || input.ends_with('.') {
        return false;
    }
    input.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    })
}

#[derive(Debug, PartialEq)]
pub enum DomainLineParsingResult {
    Skip,
    Domains(Vec<DomainName>),
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum DomainLineParsingError {
    #[error("The given line does not respect the expected format")]
    InvalidLineFormat,
    #[error("Invalid domain line format: {input}")]
    InvalidDomainFormat { input: String },
    #[error("Invalid redirect format: {input}")]
    InvalidRedirectFormat { input: String },
    #[error("Failed to read the blocklist file: {message}")]
    IoError { message: String },
}

/// Returns true if `input` consists only of digits and dots (i.e. looks
/// like an IP address rather than a domain).
pub(crate) fn is_bare_ip(input: &str) -> bool {
    !input.is_empty() && input.chars().all(|c| c.is_ascii_digit() || c == '.')
}

pub trait DomainListFormat {
    fn parse_line(line: &str) -> Result<DomainLineParsingResult, DomainLineParsingError>;
}

/// The three blocklist line formats supported by this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainListFormatKind {
    /// Hosts file format: `<redirect-ip> <hostname> <aliases[]>`.
    Hosts,
    /// One bare domain per line.
    Wildcard,
    /// Adblock Plus network rules: `||<domain>^`.
    Adblock,
}

impl DomainListFormatKind {
    /// Parses a single line with the format this kind represents.
    pub fn parse_line(self, line: &str) -> Result<DomainLineParsingResult, DomainLineParsingError> {
        match self {
            DomainListFormatKind::Hosts => HostsListStandardFormat::parse_line(line),
            DomainListFormatKind::Wildcard => WildcardDomainsFormat::parse_line(line),
            DomainListFormatKind::Adblock => AdblockFormat::parse_line(line),
        }
    }
}

/// Number of leading lines sampled by [`detect_format`].
pub const DETECTION_SAMPLE_LINES: usize = 100;

/// Detects the format of a blocklist by sampling its first lines.
///
/// Blank lines, comment lines (`#` and `!`) and the Adblock Plus header
/// marker are ignored. The detected format is the plurality winner among the
/// remaining lines: adblock must win outright, and a tie between hosts and
/// wildcard falls back to wildcard. A sample with no meaningful line defaults
/// to [`DomainListFormatKind::Wildcard`].
pub fn detect_format(content: &str) -> DomainListFormatKind {
    let mut adblock_votes: usize = 0;
    let mut hosts_votes: usize = 0;
    let mut wildcard_votes: usize = 0;

    for line in content.lines().take(DETECTION_SAMPLE_LINES) {
        let trimmed = line.trim();

        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with('!')
            || trimmed.starts_with('[')
        {
            continue;
        }

        if trimmed.starts_with("||") {
            adblock_votes += 1;
        } else if starts_with_ip(trimmed) {
            hosts_votes += 1;
        } else {
            wildcard_votes += 1;
        }
    }

    if adblock_votes > hosts_votes && adblock_votes > wildcard_votes {
        DomainListFormatKind::Adblock
    } else if hosts_votes > wildcard_votes {
        DomainListFormatKind::Hosts
    } else {
        DomainListFormatKind::Wildcard
    }
}

/// Returns true when the line starts with an IP address token (hosts format).
fn starts_with_ip(line: &str) -> bool {
    line.split_whitespace()
        .next()
        .is_some_and(|token| token.parse::<IpAddr>().is_ok())
}
