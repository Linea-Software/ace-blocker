use nutype::nutype;
use sha2::{Digest, Sha256};

/// IDs stored as directory names must be safe single components on Windows
/// and Unix; canonical catalog IDs (e.g. pack-adult-free) need not be UUIDs.
pub(crate) fn is_safe_site_pack_id(value: &str) -> bool {
    if value.is_empty()
        || value.len() > 128
        || value.starts_with('.')
        || value.ends_with('.')
        || value.contains("..")
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return false;
    }

    // Windows device basenames are reserved even with an extension.
    let basename = value.split('.').next().unwrap_or("");
    let basename = basename.to_ascii_uppercase();
    !matches!(
        basename.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

#[nutype(
    validate(predicate = is_safe_site_pack_id),
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

// Canonical pre-release site-pack version: three dot-separated unsigned
// decimal components, e.g. 1.2.3 or the dated 2026.09.02 bundle version.
// Legacy free-text, two-component and pre-release suffix formats are invalid.
fn is_site_pack_version(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.parse::<u64>().is_ok())
}

#[nutype(
    sanitize(trim),
    validate(predicate = is_site_pack_version),
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
    /// All values have exactly three validated decimal components.
    pub fn is_newer_than(&self, current: &Self) -> bool {
        let components = |version: &Self| {
            let mut segments = version.as_ref().split('.');
            std::array::from_fn::<u64, 3, _>(|_| {
                segments
                    .next()
                    .expect("validated version has three segments")
                    .parse::<u64>()
                    .expect("validated version segment is a u64")
            })
        };
        components(self) > components(current)
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
    fn site_pack_id_rejects_unsafe_path_components() {
        for invalid in [
            "",
            " ",
            ".",
            "..",
            "../pack",
            "..\\\\pack",
            "C:pack",
            "folder/pack",
            "folder\\\\pack",
            "CON",
            "con.db",
            "NUL",
            "COM1",
            "a..b",
            "a ",
            "a?",
            "name|other",
        ] {
            assert!(!is_safe_site_pack_id(invalid), "{invalid}");
            assert!(RemoteSitePackId::try_new(invalid).is_err(), "{invalid}");
        }
        for valid in [
            "pack-adult-free",
            "pack.social",
            "4f1808dc-89a7-41c5-b9a0-3f6c425ac0b9",
        ] {
            assert!(is_safe_site_pack_id(valid), "{valid}");
            assert!(RemoteSitePackId::try_new(valid).is_ok(), "{valid}");
        }
    }

    #[test]
    fn site_pack_version_owns_comparison_policy() {
        let newer = SitePackVersion::try_new("1.10.0").unwrap();
        let older = SitePackVersion::try_new("1.9.0").unwrap();
        assert!(newer.is_newer_than(&older));
        assert!(!older.is_newer_than(&newer));
        assert!(
            SitePackVersion::try_new("2026.10.01")
                .unwrap()
                .is_newer_than(&SitePackVersion::try_new("2026.09.30").unwrap())
        );
        assert!(
            !SitePackVersion::try_new("1.0.0")
                .unwrap()
                .is_newer_than(&SitePackVersion::try_new("1.0.0").unwrap())
        );
    }

    #[test]
    fn site_pack_version_rejects_legacy_and_ambiguous_forms() {
        for invalid in [
            "",
            "1.0",
            "1",
            "pack-adult-free",
            "1.0.0-beta",
            "1..0",
            "-1.0.0",
            "18446744073709551616.0.0",
        ] {
            assert!(SitePackVersion::try_new(invalid).is_err(), "{invalid}");
        }
        for valid in ["1.0.0", "2026.09.02"] {
            assert!(SitePackVersion::try_new(valid).is_ok(), "{valid}");
        }
    }
}
