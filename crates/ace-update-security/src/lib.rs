//! Fail-closed verification of full-product release metadata.
//!
//! This crate has NO installer, SCM, watchdog, or shutdown capability. Its
//! VerifiedRelease type proves only manifest authenticity, target/version
//! policy and package SHA-256 after verify_package(). Windows Authenticode,
//! protected staging and one-shot privileged maintenance are separate mandatory
//! checks before an MSI may run.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, VerifyingKey};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{self, Read},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use url::Url;

const SIGNED_CONTEXT: &[u8] = b"ace-blocker-windows-msi-release-v1\0";
const MAX_MANIFEST_BYTES: usize = 16 * 1024;
const MAX_MSI_BYTES: u64 = 4 * 1024 * 1024 * 1024;
pub const CHANNEL: &str = "stable";

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("release trust is not configured")]
    NotConfigured,
    #[error("malformed release envelope or signed payload")]
    Malformed,
    #[error("release signature is missing, invalid, or signed by an unknown key")]
    InvalidSignature,
    #[error("release targets a different platform, channel or incompatible client")]
    Incompatible,
    #[error("release version is not newer than the installed/allowed minimum")]
    NotNewer,
    #[error("release package URL is not secure")]
    UnsafeUrl,
    #[error("release certificate is not the pinned publisher certificate")]
    PublisherMismatch,
    #[error("package SHA-256 does not match the signed manifest")]
    HashMismatch,
    #[error("package is too large")]
    PackageTooLarge,
    #[error("package read error: {0}")]
    Io(#[from] io::Error),
    #[error("another update is in progress")]
    Busy,
    #[error("invalid update state transition")]
    InvalidTransition,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SignedEnvelope {
    /// Base64 of the exact UTF-8 JSON bytes that were signed (not reserialized).
    pub payload: String,
    /// Base64 of the detached 64-byte Ed25519 signature.
    pub signature: String,
    pub key_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReleaseMetadata {
    pub schema_version: u8,
    pub version: String,
    pub channel: String,
    pub os: String,
    pub architecture: String,
    pub package_url: String,
    pub sha256: String,
    /// SHA-256 fingerprint of the DER-encoded Authenticode signer certificate.
    pub publisher_certificate_sha256: String,
    pub minimum_installed_version: Option<String>,
}

/// Pinning policy is supplied by the signed desktop binary, never by the
/// network response. No embedded key => automatic installation disabled.
pub struct TrustPolicy {
    key_id: String,
    manifest_key: VerifyingKey,
    publisher_cert_sha256: [u8; 32],
}

impl TrustPolicy {
    pub fn from_hex(
        key_id: &str,
        manifest_public_key_hex: &str,
        publisher_certificate_sha256_hex: &str,
    ) -> Result<Self, UpdateError> {
        if key_id.is_empty()
            || key_id.len() > 80
            || !key_id
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
        {
            return Err(UpdateError::NotConfigured);
        }
        let public_key_bytes =
            hex32(manifest_public_key_hex).map_err(|_| UpdateError::NotConfigured)?;
        let manifest_key =
            VerifyingKey::from_bytes(&public_key_bytes).map_err(|_| UpdateError::NotConfigured)?;
        let publisher_cert_sha256 =
            hex32(publisher_certificate_sha256_hex).map_err(|_| UpdateError::NotConfigured)?;
        if publisher_cert_sha256 == [0u8; 32] {
            return Err(UpdateError::NotConfigured);
        }
        Ok(Self {
            key_id: key_id.to_owned(),
            manifest_key,
            publisher_cert_sha256,
        })
    }

    /// Compile-time trust roots. Both the public key and Windows publisher
    /// certificate fingerprint must be supplied by the production build.
    /// The private signing key must NEVER be a build variable or repository file.
    pub fn embedded() -> Result<Self, UpdateError> {
        Self::from_hex(
            "ace-stable-v1",
            option_env!("ACE_RELEASE_MANIFEST_PUBLIC_KEY_HEX").ok_or(UpdateError::NotConfigured)?,
            option_env!("ACE_RELEASE_PUBLISHER_CERT_SHA256").ok_or(UpdateError::NotConfigured)?,
        )
    }
}

fn hex32(value: &str) -> Result<[u8; 32], UpdateError> {
    if value.len() != 64 || !value.bytes().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(UpdateError::Malformed);
    }
    let decoded = hex::decode(value).map_err(|_| UpdateError::Malformed)?;
    decoded.try_into().map_err(|_| UpdateError::Malformed)
}

/// Successful release authentication is represented by a non-constructible
/// witness; a caller cannot obtain this value from an unverified JSON body.
#[derive(Clone)]
pub struct VerifiedRelease {
    metadata: ReleaseMetadata,
    version: Version,
    digest: [u8; 32],
}

impl VerifiedRelease {
    pub fn metadata(&self) -> &ReleaseMetadata {
        &self.metadata
    }
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// Stream and hash an MSI. Does NOT claim that Authenticode has passed.
    pub fn verify_package<R: Read>(&self, mut reader: R) -> Result<(), UpdateError> {
        let mut hasher = Sha256::new();
        let mut total_bytes = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            total_bytes = total_bytes
                .checked_add(read as u64)
                .filter(|size| *size <= MAX_MSI_BYTES)
                .ok_or(UpdateError::PackageTooLarge)?;
            hasher.update(&buffer[..read]);
        }
        let actual: [u8; 32] = hasher.finalize().into();
        if actual != self.digest {
            return Err(UpdateError::HashMismatch);
        }
        Ok(())
    }
}

/// Checks signature BEFORE parsing the signed payload and applying policy.
/// The version floor must be stored in privileged persistent state if used
/// to block previously revoked/vulnerable versions across installations.
pub fn verify_release(
    envelope_json: &[u8],
    policy: &TrustPolicy,
    installed: &Version,
    version_floor: &Version,
) -> Result<VerifiedRelease, UpdateError> {
    if envelope_json.len() > MAX_MANIFEST_BYTES {
        return Err(UpdateError::Malformed);
    }
    let envelope: SignedEnvelope =
        serde_json::from_slice(envelope_json).map_err(|_| UpdateError::Malformed)?;
    if envelope.key_id != policy.key_id {
        return Err(UpdateError::InvalidSignature);
    }
    let payload = STANDARD
        .decode(envelope.payload.as_bytes())
        .map_err(|_| UpdateError::Malformed)?;
    if payload.len() > MAX_MANIFEST_BYTES {
        return Err(UpdateError::Malformed);
    }
    let signature_bytes = STANDARD
        .decode(envelope.signature.as_bytes())
        .map_err(|_| UpdateError::InvalidSignature)?;
    let signature =
        Signature::from_slice(&signature_bytes).map_err(|_| UpdateError::InvalidSignature)?;
    let mut signed_bytes = Vec::with_capacity(SIGNED_CONTEXT.len() + payload.len());
    signed_bytes.extend_from_slice(SIGNED_CONTEXT);
    signed_bytes.extend_from_slice(&payload);
    policy
        .manifest_key
        .verify_strict(&signed_bytes, &signature)
        .map_err(|_| UpdateError::InvalidSignature)?;

    let metadata: ReleaseMetadata =
        serde_json::from_slice(&payload).map_err(|_| UpdateError::Malformed)?;
    let version = Version::parse(&metadata.version).map_err(|_| UpdateError::Malformed)?;
    if metadata.schema_version != 1
        || metadata.channel != CHANNEL
        || metadata.os != "windows"
        || metadata.architecture != "x64"
        || !version.pre.is_empty()
        || !version.build.is_empty()
    {
        return Err(UpdateError::Incompatible);
    }
    if version <= *installed || version <= *version_floor {
        return Err(UpdateError::NotNewer);
    }
    if let Some(minimum) = &metadata.minimum_installed_version {
        let minimum = Version::parse(minimum).map_err(|_| UpdateError::Malformed)?;
        if installed < &minimum {
            return Err(UpdateError::Incompatible);
        }
    }
    let package_url = Url::parse(&metadata.package_url).map_err(|_| UpdateError::UnsafeUrl)?;
    if package_url.scheme() != "https"
        || package_url.host_str().is_none()
        || !package_url.username().is_empty()
        || package_url.password().is_some()
        || package_url.fragment().is_some()
    {
        return Err(UpdateError::UnsafeUrl);
    }
    if hex32(&metadata.publisher_certificate_sha256)? != policy.publisher_cert_sha256 {
        return Err(UpdateError::PublisherMismatch);
    }
    let digest = hex32(&metadata.sha256)?;
    Ok(VerifiedRelease {
        metadata,
        version,
        digest,
    })
}

/// State transitions are explicit so UI and future privileged orchestration
/// cannot equate "process started" with successful enforcement recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdatePhase {
    Idle,
    Checking,
    Available,
    Downloading,
    Verifying,
    Ready,
    PreparingMaintenance,
    Installing,
    Restarting,
    VerifyingHealth,
    Succeeded,
    Failed,
    RecoveryRequired,
}

impl UpdatePhase {
    pub fn can_advance(self, next: Self) -> bool {
        use UpdatePhase::*;
        matches!(
            (self, next),
            (Idle | Succeeded | Failed | RecoveryRequired, Checking)
                | (Checking, Available | Failed | Idle)
                | (Available, Downloading | Failed)
                | (Downloading, Verifying | Failed)
                | (Verifying, Ready | Failed)
                | (Ready, PreparingMaintenance | Failed)
                | (PreparingMaintenance, Installing | Failed | RecoveryRequired)
                | (Installing, Restarting | RecoveryRequired)
                | (Restarting, VerifyingHealth | RecoveryRequired)
                | (VerifyingHealth, Succeeded | RecoveryRequired)
        )
    }

    /// Conservative classification after a simulated crash/restart.
    pub fn after_interruption(self) -> Self {
        use UpdatePhase::*;
        match self {
            PreparingMaintenance | Installing | Restarting | VerifyingHealth => RecoveryRequired,
            Checking | Available | Downloading | Verifying | Ready => Failed,
            otherwise => otherwise,
        }
    }
}

/// Single-flight coordination for one process. A future installer must also
/// acquire a system-wide privileged mutex across processes/reboots.
#[derive(Clone)]
pub struct UpdateCoordinator(Arc<CoordinatorInner>);
struct CoordinatorInner {
    active: AtomicBool,
    phase: Mutex<UpdatePhase>,
}

impl Default for UpdateCoordinator {
    fn default() -> Self {
        Self(Arc::new(CoordinatorInner {
            active: AtomicBool::new(false),
            phase: Mutex::new(UpdatePhase::Idle),
        }))
    }
}

impl UpdateCoordinator {
    pub fn phase(&self) -> UpdatePhase {
        *self.0.phase.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn try_begin(&self) -> Result<UpdateSession, UpdateError> {
        if self
            .0
            .active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(UpdateError::Busy);
        }
        *self.0.phase.lock().unwrap_or_else(|e| e.into_inner()) = UpdatePhase::Checking;
        Ok(UpdateSession {
            coordinator: self.clone(),
        })
    }
}

pub struct UpdateSession {
    coordinator: UpdateCoordinator,
}

impl UpdateSession {
    pub fn advance(&self, next: UpdatePhase) -> Result<(), UpdateError> {
        let mut current = self
            .coordinator
            .0
            .phase
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if !current.can_advance(next) {
            return Err(UpdateError::InvalidTransition);
        }
        *current = next;
        Ok(())
    }
}

impl Drop for UpdateSession {
    fn drop(&mut self) {
        let mut current = self
            .coordinator
            .0
            .phase
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *current = current.after_interruption();
        self.coordinator.0.active.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    const TEST_KEY_BYTES: [u8; 32] = [17; 32];
    // Test key is deliberately deterministic and isolated to #[cfg(test)].
    fn policy() -> TrustPolicy {
        TrustPolicy::from_hex(
            "ace-stable-v1",
            &hex::encode(
                SigningKey::from_bytes(&TEST_KEY_BYTES)
                    .verifying_key()
                    .to_bytes(),
            ),
            &"42".repeat(32),
        )
        .unwrap()
    }
    fn metadata(version: &str) -> ReleaseMetadata {
        ReleaseMetadata {
            schema_version: 1,
            version: version.into(),
            channel: "stable".into(),
            os: "windows".into(),
            architecture: "x64".into(),
            package_url: "https://aceblocker.com/download/AceBlocker-2.0.0.msi".into(),
            sha256: hex::encode(Sha256::digest(b"test-msi")),
            publisher_certificate_sha256: "42".repeat(32),
            minimum_installed_version: None,
        }
    }
    fn signed(metadata: &ReleaseMetadata) -> Vec<u8> {
        let payload = serde_json::to_vec(metadata).unwrap();
        let mut signing_input = SIGNED_CONTEXT.to_vec();
        signing_input.extend_from_slice(&payload);
        let signature = SigningKey::from_bytes(&TEST_KEY_BYTES).sign(&signing_input);
        serde_json::to_vec(&SignedEnvelope {
            payload: STANDARD.encode(payload),
            signature: STANDARD.encode(signature.to_bytes()),
            key_id: "ace-stable-v1".into(),
        })
        .unwrap()
    }
    fn verified(payload: &[u8]) -> Result<VerifiedRelease, UpdateError> {
        verify_release(
            payload,
            &policy(),
            &Version::new(1, 0, 0),
            &Version::new(1, 0, 0),
        )
    }

    #[test]
    fn accepts_authentic_newer_release_and_package_hash() {
        let release = verified(&signed(&metadata("2.0.0"))).unwrap();
        assert_eq!(release.version(), &Version::new(2, 0, 0));
        release.verify_package(&b"test-msi"[..]).unwrap();
        assert!(matches!(
            release.verify_package(&b"wrong-msi"[..]),
            Err(UpdateError::HashMismatch)
        ));
    }
    #[test]
    fn rejects_downgrade_and_persisted_version_floor() {
        assert!(matches!(
            verified(&signed(&metadata("0.9.99"))),
            Err(UpdateError::NotNewer)
        ));
        assert!(matches!(
            verify_release(
                &signed(&metadata("2.0.0")),
                &policy(),
                &Version::new(1, 0, 0),
                &Version::new(3, 0, 0)
            ),
            Err(UpdateError::NotNewer)
        ));
    }
    #[test]
    fn rejects_unsigned_malformed_and_tampered_manifest() {
        assert!(matches!(verified(br#"{}"#), Err(UpdateError::Malformed)));
        let mut envelope: SignedEnvelope =
            serde_json::from_slice(&signed(&metadata("2.0.0"))).unwrap();
        envelope.signature = "".into();
        assert!(matches!(
            verified(&serde_json::to_vec(&envelope).unwrap()),
            Err(UpdateError::InvalidSignature)
        ));
        envelope = serde_json::from_slice(&signed(&metadata("2.0.0"))).unwrap();
        envelope.payload = STANDARD.encode(serde_json::to_vec(&metadata("4.0.0")).unwrap());
        assert!(matches!(
            verified(&serde_json::to_vec(&envelope).unwrap()),
            Err(UpdateError::InvalidSignature)
        ));
        envelope.key_id = "unknown-key".into();
        assert!(matches!(
            verified(&serde_json::to_vec(&envelope).unwrap()),
            Err(UpdateError::InvalidSignature)
        ));
    }
    #[test]
    fn rejects_invalid_target_url_cert_and_version_requirements() {
        let mut release = metadata("2.0.0");
        release.package_url = "http://example.com/installer.msi".into();
        assert!(matches!(
            verified(&signed(&release)),
            Err(UpdateError::UnsafeUrl)
        ));
        release = metadata("2.0.0");
        release.publisher_certificate_sha256 = "11".repeat(32);
        assert!(matches!(
            verified(&signed(&release)),
            Err(UpdateError::PublisherMismatch)
        ));
        release = metadata("2.0.0");
        release.minimum_installed_version = Some("1.1.0".into());
        assert!(matches!(
            verified(&signed(&release)),
            Err(UpdateError::Incompatible)
        ));
        release = metadata("2.0.0");
        release.channel = "beta".into();
        assert!(matches!(
            verified(&signed(&release)),
            Err(UpdateError::Incompatible)
        ));
        release = metadata("2.0.0");
        release.version = "2.0.0-rc1".into();
        assert!(matches!(
            verified(&signed(&release)),
            Err(UpdateError::Incompatible)
        ));
    }
    #[test]
    fn rejects_missing_release_trust_configuration() {
        assert!(matches!(
            TrustPolicy::from_hex("", "", ""),
            Err(UpdateError::NotConfigured)
        ));
    }
    #[test]
    fn update_is_single_flight_and_interruption_is_classified() {
        let coordinator = UpdateCoordinator::default();
        let session = coordinator.try_begin().unwrap();
        assert!(matches!(coordinator.try_begin(), Err(UpdateError::Busy)));
        session.advance(UpdatePhase::Available).unwrap();
        session.advance(UpdatePhase::Downloading).unwrap();
        assert!(matches!(
            session.advance(UpdatePhase::Succeeded),
            Err(UpdateError::InvalidTransition)
        ));
        drop(session);
        assert_eq!(coordinator.phase(), UpdatePhase::Failed);
        let session = coordinator.try_begin().unwrap();
        session.advance(UpdatePhase::Available).unwrap();
        session.advance(UpdatePhase::Downloading).unwrap();
        session.advance(UpdatePhase::Verifying).unwrap();
        session.advance(UpdatePhase::Ready).unwrap();
        session.advance(UpdatePhase::PreparingMaintenance).unwrap();
        session.advance(UpdatePhase::Installing).unwrap();
        drop(session);
        assert_eq!(coordinator.phase(), UpdatePhase::RecoveryRequired);
    }
    #[test]
    fn completed_update_remains_successful_after_session_close() {
        let coordinator = UpdateCoordinator::default();
        let session = coordinator.try_begin().unwrap();
        for next in [
            UpdatePhase::Available,
            UpdatePhase::Downloading,
            UpdatePhase::Verifying,
            UpdatePhase::Ready,
            UpdatePhase::PreparingMaintenance,
            UpdatePhase::Installing,
            UpdatePhase::Restarting,
            UpdatePhase::VerifyingHealth,
            UpdatePhase::Succeeded,
        ] {
            session.advance(next).unwrap();
        }
        drop(session);
        assert_eq!(coordinator.phase(), UpdatePhase::Succeeded);
    }
    #[test]
    fn read_failure_propagates_without_claiming_success() {
        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(
                    io::ErrorKind::ConnectionReset,
                    "interrupted download",
                ))
            }
        }
        let release = verified(&signed(&metadata("2.0.0"))).unwrap();
        assert!(matches!(
            release.verify_package(Broken),
            Err(UpdateError::Io(_))
        ));
    }
}
