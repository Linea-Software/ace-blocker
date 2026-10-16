//! Parsing of domain blocklist files in hosts, wildcard and Adblock formats.

pub mod format;

pub use format::{
    AdblockFormat, DETECTION_SAMPLE_LINES, DomainLineParsingError, DomainLineParsingResult,
    DomainListFormat, DomainListFormatKind, DomainName, HostsListStandardFormat,
    WildcardDomainsFormat, detect_format,
};

#[cfg(test)]
mod tests {
    use super::*;

    // ── detect_format ───────────────────────────────────────────────

    #[test]
    fn detect_format_identifies_hosts() {
        assert_eq!(
            detect_format("0.0.0.0 example.com\n127.0.0.1 localhost\n0.0.0.0 a.com\n"),
            DomainListFormatKind::Hosts
        );
    }

    #[test]
    fn detect_format_identifies_wildcard() {
        assert_eq!(
            detect_format("example.com\nads.example.com\nwww.00fun.com\n"),
            DomainListFormatKind::Wildcard
        );
    }

    #[test]
    fn detect_format_identifies_adblock() {
        assert_eq!(
            detect_format("[Adblock Plus]\n! Title: Light\n||example.com^\n||ads.example.com^\n"),
            DomainListFormatKind::Adblock
        );
    }

    #[test]
    fn detect_format_ignores_comments() {
        // A sample made only of comments carries no signal: falls back to
        // Wildcard.
        assert_eq!(
            detect_format("# Title: x\n# Version: 1\n! comment\n"),
            DomainListFormatKind::Wildcard
        );
    }

    #[test]
    fn detect_format_prefers_the_majority() {
        let sample =
            "example.com\nother.com\n0.0.0.0 one.com\n0.0.0.0 two.com\n0.0.0.0 three.com\n";
        assert_eq!(detect_format(sample), DomainListFormatKind::Hosts);
    }

    #[test]
    fn detect_format_wildcard_wins_hosts_tie() {
        assert_eq!(
            detect_format("example.com\n0.0.0.0 one.com\n"),
            DomainListFormatKind::Wildcard
        );
    }
}
