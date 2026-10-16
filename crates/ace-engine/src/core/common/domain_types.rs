use nutype::nutype;
use sha2::{Digest, Sha256};

#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        TryFrom,
        FromStr,
        Display,
        Deref,
        AsRef,
        Serialize,
        Deserialize
    )
)]
pub struct RemoteSitePackId(String);

#[nutype(
    sanitize(trim, lowercase),
    validate(predicate = is_sha256_hex),
    derive(
        Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, TryFrom, FromStr, Display, Deref,
        AsRef, Serialize, Deserialize
    )
)]
pub struct Sha256Digest(String);

impl Sha256Digest {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        Self::try_new(hex::encode(digest)).expect("hex-encoded SHA-256 digest is always valid")
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
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
        PartialOrd,
        Ord,
        TryFrom,
        FromStr,
        Display,
        Deref,
        AsRef,
        Serialize,
        Deserialize
    )
)]
pub struct SitePackVersion(String);

impl SitePackVersion {
    /// Compares dotted version strings numerically by segment and falls back to
    /// lexicographic ordering when either corresponding segment is non-numeric.
    pub fn is_newer_than(&self, current: &Self) -> bool {
        let candidate = self.as_ref();
        let current = current.as_ref();
        let candidate_parts: Vec<&str> = candidate.split('.').collect();
        let current_parts: Vec<&str> = current.split('.').collect();

        for (candidate_part, current_part) in candidate_parts.iter().zip(current_parts.iter()) {
            match (candidate_part.parse::<u64>(), current_part.parse::<u64>()) {
                (Ok(candidate_number), Ok(current_number)) => {
                    if candidate_number != current_number {
                        return candidate_number > current_number;
                    }
                }
                _ => return candidate > current,
            }
        }

        candidate_parts.len() > current_parts.len()
    }

    pub fn is_bundle_style(&self) -> bool {
        let parts: Vec<&str> = self.as_ref().split('.').collect();
        if parts.len() != 3 || parts[0].len() != 4 {
            return false;
        }
        parts.iter().all(|part| part.parse::<u64>().is_ok())
    }
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
        PartialOrd,
        Ord,
        TryFrom,
        FromStr,
        Display,
        Deref,
        AsRef,
        Serialize,
        Deserialize
    )
)]
pub struct PackDbFormatVersion(String);

#[nutype(
    validate(greater_or_equal = 1),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        TryFrom,
        Display,
        Serialize,
        Deserialize
    )
)]
pub struct BundleSchemaVersion(u32);

#[nutype(
    validate(greater_or_equal = 1),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        TryFrom,
        Display,
        Serialize,
        Deserialize
    )
)]
pub struct ApiPort(u16);

/// API keys intentionally do not implement `Debug` or `Display`, which makes
/// accidental secret exposure through routine logging/formatting harder.
#[nutype(
    validate(not_empty),
    derive(Clone, PartialEq, Eq, Hash, TryFrom, FromStr, AsRef)
)]
pub struct ApiKey(String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_digest_normalizes_and_validates() {
        let digest = Sha256Digest::try_new("A".repeat(64)).unwrap();
        assert_eq!(digest.as_ref(), "a".repeat(64));
        assert!(Sha256Digest::try_new("xyz").is_err());
    }

    #[test]
    fn api_port_rejects_zero() {
        assert!(ApiPort::try_new(0).is_err());
        assert_eq!(ApiPort::try_new(10567).unwrap().into_inner(), 10567);
    }

    #[test]
    fn api_key_rejects_empty_value() {
        assert!(ApiKey::try_new(String::new()).is_err());
    }

    #[test]
    fn api_key_accepts_opaque_and_prefixed_values_without_fixed_length() {
        for value in ["opaque-value", "custom_prefix_opaque-value"] {
            let api_key = ApiKey::try_new(value.to_string()).unwrap();
            assert_eq!(api_key.as_ref(), value);
        }
    }

    #[test]
    fn api_key_preserves_value_exactly_without_normalization() {
        let value = "  MixedCase.Value-123_\t";
        let api_key = ApiKey::try_new(value.to_string()).unwrap();

        assert_eq!(api_key.as_ref(), value);
    }

    #[test]
    fn site_pack_version_owns_comparison_policy() {
        let newer = SitePackVersion::try_new("1.10").unwrap();
        let older = SitePackVersion::try_new("1.9").unwrap();
        assert!(newer.is_newer_than(&older));
        assert!(!older.is_newer_than(&newer));
        assert!(
            SitePackVersion::try_new("2026.09.02")
                .unwrap()
                .is_bundle_style()
        );
        assert!(!SitePackVersion::try_new("1.0").unwrap().is_bundle_style());
    }
}
