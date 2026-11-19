//! Password-based client-side encryption for cloud backups.
//!
//! The account password is supplied only for an explicit upload/restore. No
//! long-lived backup key is stored by the engine and the plaintext EngineBackup
//! is never sent to the Ace Blocker site. This does NOT make the existing
//! Better Auth password-login protocol zero knowledge.
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::manager::network::backup::EngineBackup;
use crate::service::api::error::ApiError;

const AAD: &[u8] = b"ace-blocker-cloud-backup:aes-256-gcm:argon2id:v1";
const ARGON_MEMORY_KIB: u32 = 65_536;
const ARGON_ITERATIONS: u32 = 3;
const ARGON_PARALLELISM: u32 = 1;
const MAX_CIPHERTEXT_BYTES: usize = 2 * 1024 * 1024;
const FORMAT: &str = "ace-cloud-e2ee";
const ALGORITHM: &str = "aes-256-gcm";
const KDF: &str = "argon2id";

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncryptedCloudBackup {
    pub format: String,
    pub format_version: u8,
    pub algorithm: String,
    pub kdf: String,
    pub salt: String,
    pub nonce: String,
    pub ciphertext: String,
}

/// A password is required for every explicit cloud upload and restore.
/// Do not derive Debug or log this request in RPC/engine diagnostics.
#[derive(Serialize, Deserialize, schemars::JsonSchema)]
pub struct CloudBackupPasswordRequest {
    pub password: String,
}

fn derivation(password: &str, salt: &[u8; 16]) -> Result<Zeroizing<[u8; 32]>, ApiError> {
    if password.is_empty() {
        return Err(ApiError::BadRequest("Account password is required".into()));
    }
    let params = Params::new(
        ARGON_MEMORY_KIB,
        ARGON_ITERATIONS,
        ARGON_PARALLELISM,
        Some(32),
    )
    .map_err(|_| ApiError::Internal("Could not configure backup key derivation".into()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0_u8; 32]);
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut *key)
        .map_err(|_| ApiError::Internal("Could not derive backup encryption key".into()))?;
    Ok(key)
}

fn decode<const LENGTH: usize>(encoded: &str) -> Result<[u8; LENGTH], ApiError> {
    let raw = STANDARD
        .decode(encoded)
        .map_err(|_| ApiError::BadRequest("Invalid encrypted backup format".into()))?;
    if raw.len() != LENGTH || STANDARD.encode(&raw) != encoded {
        return Err(ApiError::BadRequest(
            "Invalid encrypted backup format".into(),
        ));
    }
    raw.try_into()
        .map_err(|_| ApiError::BadRequest("Invalid encrypted backup format".into()))
}

pub fn encrypt_backup(
    backup: &EngineBackup,
    password: &str,
) -> Result<EncryptedCloudBackup, ApiError> {
    let salt: [u8; 16] = rand::random();
    let nonce: [u8; 12] = rand::random();
    let key = derivation(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&*key)
        .map_err(|_| ApiError::Internal("Could not initialize backup encryption".into()))?;
    let plaintext = Zeroizing::new(
        serde_json::to_vec(backup)
            .map_err(|_| ApiError::Internal("Could not serialize backup".into()))?,
    );
    let encrypted = cipher
        .encrypt(
            &Nonce::from(nonce),
            Payload {
                msg: &plaintext,
                aad: AAD,
            },
        )
        .map_err(|_| ApiError::Internal("Could not encrypt backup".into()))?;
    if encrypted.len() > MAX_CIPHERTEXT_BYTES {
        return Err(ApiError::BadRequest("Encrypted backup exceeds 2MB".into()));
    }
    Ok(EncryptedCloudBackup {
        format: FORMAT.into(),
        format_version: 1,
        algorithm: ALGORITHM.into(),
        kdf: KDF.into(),
        salt: STANDARD.encode(salt),
        nonce: STANDARD.encode(nonce),
        ciphertext: STANDARD.encode(encrypted),
    })
}

pub fn decrypt_backup(
    encrypted: &EncryptedCloudBackup,
    password: &str,
) -> Result<EngineBackup, ApiError> {
    if encrypted.format != FORMAT
        || encrypted.format_version != 1
        || encrypted.algorithm != ALGORITHM
        || encrypted.kdf != KDF
    {
        return Err(ApiError::BadRequest(
            "Unsupported encrypted backup format".into(),
        ));
    }
    let salt = decode::<16>(&encrypted.salt)?;
    let nonce = decode::<12>(&encrypted.nonce)?;
    let ciphertext = STANDARD
        .decode(&encrypted.ciphertext)
        .map_err(|_| ApiError::BadRequest("Invalid encrypted backup format".into()))?;
    if ciphertext.len() < 16
        || ciphertext.len() > MAX_CIPHERTEXT_BYTES
        || STANDARD.encode(&ciphertext) != encrypted.ciphertext
    {
        return Err(ApiError::BadRequest(
            "Invalid encrypted backup format".into(),
        ));
    }

    let key = derivation(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&*key)
        .map_err(|_| ApiError::Internal("Could not initialize backup decryption".into()))?;
    let plaintext = Zeroizing::new(
        cipher
            .decrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: &ciphertext,
                    aad: AAD,
                },
            )
            .map_err(|_| {
                ApiError::BadRequest("Incorrect password or corrupted cloud backup".into())
            })?,
    );
    serde_json::from_slice::<EngineBackup>(&plaintext)
        .map_err(|_| ApiError::BadRequest("Invalid decrypted backup".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_backup() -> EngineBackup {
        EngineBackup {
            version: "1.0.0".into(),
            custom_site_packs: vec![],
            network_blocks: vec![],
            deactivated_resistance_scripts: vec!["private.test".into()],
            activated_resistance_scripts: vec![],
        }
    }

    #[test]
    fn roundtrip_uses_random_salt_nonce_and_does_not_expose_backup() {
        let backup = sample_backup();
        let first = encrypt_backup(&backup, "a valid account password").unwrap();
        let second = encrypt_backup(&backup, "a valid account password").unwrap();
        assert_ne!(first.salt, second.salt);
        assert_ne!(first.nonce, second.nonce);
        assert_ne!(first.ciphertext, second.ciphertext);
        assert!(
            !serde_json::to_string(&first)
                .unwrap()
                .contains("private.test")
        );
        assert_eq!(
            serde_json::to_value(decrypt_backup(&first, "a valid account password").unwrap())
                .unwrap(),
            serde_json::to_value(backup).unwrap()
        );
    }

    #[test]
    fn wrong_password_and_ciphertext_tampering_fail_closed() {
        let backup = sample_backup();
        let mut encrypted = encrypt_backup(&backup, "correct password").unwrap();
        assert!(decrypt_backup(&encrypted, "incorrect password").is_err());
        let mut data = STANDARD.decode(&encrypted.ciphertext).unwrap();
        data[0] ^= 1;
        encrypted.ciphertext = STANDARD.encode(data);
        assert!(decrypt_backup(&encrypted, "correct password").is_err());
    }

    #[test]
    fn full_backup_preserves_timezone_icons_activation_and_script_overrides() {
        use crate::core::{ScheduleMode, UserDefinedRule, UserDefinedRuleType};
        use crate::manager::network::backup::{
            BackupNetworkBlock, BackupScheduleEntry, BackupSitePack, BackupSitePackSource,
        };

        let mut backup = sample_backup();
        backup.custom_site_packs.push(BackupSitePack {
            id: "pack-custom".into(),
            remote_id: None,
            requires_pro: false,
            version: Some("2.0".into()),
            name: "Private catalog".into(),
            description: Some("Sensitive user site list".into()),
            icon: Some("moon".into()),
            sources: vec![BackupSitePackSource {
                source: "https://example.test/sensitive-path".into(),
                source_type: "url".into(),
                format: "hosts".into(),
            }],
        });
        backup.network_blocks.push(BackupNetworkBlock {
            id: uuid::Uuid::new_v4(),
            name: "Night restrictions".into(),
            enabled: false,
            schedule_mode: ScheduleMode::Scheduled,
            safe_search: true,
            icon: Some("shield".into()),
            schedule_timezone: "Europe/Rome".into(),
            schedules: vec![BackupScheduleEntry {
                start_time: "22:00".into(),
                end_time: "06:00".into(),
                monday: true,
                tuesday: false,
                wednesday: true,
                thursday: false,
                friday: true,
                saturday: false,
                sunday: true,
            }],
            user_rules: vec![UserDefinedRule::new(
                "example.com".into(),
                UserDefinedRuleType::Domain,
            )],
            connected_site_packs: vec!["pack-custom".into()],
        });
        backup
            .activated_resistance_scripts
            .push("com.site.opt_in".into());
        let encrypted = encrypt_backup(&backup, "account password").unwrap();
        let actual = decrypt_backup(&encrypted, "account password").unwrap();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(backup).unwrap()
        );
    }

    #[test]
    fn empty_password_is_rejected() {
        assert!(encrypt_backup(&sample_backup(), "").is_err());
    }
}
