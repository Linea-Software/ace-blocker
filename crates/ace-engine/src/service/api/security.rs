//! Capability token comparison used by the remoc RPC boundary.

pub(super) fn tokens_match(presented: &str, expected: &str) -> bool {
    // Compare fixed-size SHA-256 digests without short-circuiting equality.
    // Hashing still takes time proportional to each token's length.
    use sha2::{Digest, Sha256};
    use subtle::ConstantTimeEq;
    let presented_digest = Sha256::digest(presented.as_bytes());
    let expected_digest = Sha256::digest(expected.as_bytes());
    presented_digest
        .as_slice()
        .ct_eq(expected_digest.as_slice())
        .into()
}

#[cfg(test)]
mod tests {
    use super::tokens_match;
    #[test]
    fn compares_capability_token() {
        assert!(tokens_match("secret", "secret"));
        assert!(!tokens_match("secret", "secret2"));
        assert!(!tokens_match("", "secret"));
        assert!(tokens_match("", ""));
        assert!(!tokens_match("secret", "SECRET"));
    }
}
