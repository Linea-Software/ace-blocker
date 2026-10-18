//! Capability token comparison used by the remoc RPC boundary.

pub(super) fn tokens_match(presented: &str, expected: &str) -> bool {
    // Compare SHA-256 digests: constant time in the input length and avoids
    // leaking the expected token through timing or length.
    use sha2::{Digest, Sha256};
    let presented_digest = hex::encode(Sha256::digest(presented.as_bytes()));
    let expected_digest = hex::encode(Sha256::digest(expected.as_bytes()));
    presented_digest == expected_digest
}

#[cfg(test)]
mod tests {
    use super::tokens_match;
    #[test]
    fn compares_capability_token() {
        assert!(tokens_match("secret", "secret"));
        assert!(!tokens_match("secret", "secret2"));
        assert!(!tokens_match("", "secret"));
    }
}
