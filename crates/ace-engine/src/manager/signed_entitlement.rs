//! Offline Pro lease verification. The public verification key is obtained
//! from the authenticated entitlement endpoint over HTTPS (or loopback HTTP
//! in local development) and persisted alongside the server-signed lease.
//! This keeps dev and release behavior aligned without a build-time key.
//! The private signing key never leaves the server.
use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub const OFFLINE_GRACE_SECONDS: i64 = 72 * 3600;
pub const LEASE_SECONDS: i64 = 24 * 3600;
const REQUIRED_FEATURES: [&str; 4] = [
    "custom_site_packs",
    "premium_site_packs",
    "resistance_mode",
    "cloud_backup",
];

#[derive(Debug, Deserialize)]
struct Header {
    alg: String,
    typ: String,
    kid: String,
}

#[derive(Debug, Deserialize)]
struct Claims {
    iss: String,
    sub: String,
    key_sha256: String,
    iat: i64,
    exp: i64,
    offline_until: i64,
    features: Vec<String>,
}

/// Validate an Ed25519 verification key received from the account server.
pub fn validate_public_key(public_key_base64: &str) -> Result<(), String> {
    let raw: [u8; 32] = STANDARD
        .decode(public_key_base64)
        .map_err(|_| "invalid verification key encoding")?
        .try_into()
        .map_err(|_| "invalid verification key length")?;
    VerifyingKey::from_bytes(&raw).map_err(|_| "invalid Ed25519 verification key")?;
    Ok(())
}

/// Only the upper expiry is returned; a token never grants indefinite Pro.
/// The verification key comes from the same authenticated origin that issued
/// the lease, not a client-supplied IPC argument.
pub fn verify_lease(
    token: &str,
    api_key: &str,
    now: i64,
    public_key_base64: &str,
) -> Result<i64, String> {
    let mut parts = token.split('.');
    let (Some(header), Some(payload), Some(signature), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err("invalid entitlement shape".into());
    };
    validate_public_key(public_key_base64)?;
    let public_key: [u8; 32] = STANDARD
        .decode(public_key_base64)
        .map_err(|_| "invalid verification key encoding")?
        .try_into()
        .map_err(|_| "invalid verification key length")?;
    let verifying_key =
        VerifyingKey::from_bytes(&public_key).map_err(|_| "invalid verification key")?;
    let signature = Signature::from_slice(
        &URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| "invalid signature encoding")?,
    )
    .map_err(|_| "invalid signature length")?;
    verifying_key
        .verify_strict(format!("{header}.{payload}").as_bytes(), &signature)
        .map_err(|_| "entitlement signature mismatch")?;
    let header: Header = serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(header)
            .map_err(|_| "invalid header encoding")?,
    )
    .map_err(|_| "invalid header")?;
    if header.alg != "EdDSA" || header.typ != "ACE-ENTITLEMENT" || header.kid != "ace-v1" {
        return Err("unsupported entitlement header".into());
    }
    let claims: Claims = serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| "invalid payload encoding")?,
    )
    .map_err(|_| "invalid payload")?;
    if claims.iss != "aceblocker.com" || claims.sub.is_empty() {
        return Err("untrusted issuer or subject".into());
    }
    let fingerprint = hex::encode(Sha256::digest(api_key.as_bytes()));
    if claims.key_sha256 != fingerprint {
        return Err("entitlement bound to another API key".into());
    }
    if claims.iat > now + 300
        || claims.exp <= claims.iat
        || claims.exp - claims.iat > LEASE_SECONDS
        || claims.offline_until != claims.exp + OFFLINE_GRACE_SECONDS
        || now > claims.offline_until
    {
        return Err("entitlement is expired or has invalid lifetime".into());
    }
    if !REQUIRED_FEATURES
        .iter()
        .all(|required| claims.features.iter().any(|value| value == required))
    {
        return Err("missing Pro feature grants".into());
    }
    Ok(claims.offline_until)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::json;

    fn signed_fixture(now: i64) -> (String, String) {
        let signing = SigningKey::from_bytes(&[17; 32]);
        let pub_key = STANDARD.encode(signing.verifying_key().as_bytes());
        let header = URL_SAFE_NO_PAD.encode(
            json!({
                "alg": "EdDSA", "typ": "ACE-ENTITLEMENT", "kid": "ace-v1"
            })
            .to_string(),
        );
        let payload = URL_SAFE_NO_PAD.encode(
            json!({
                "iss": "aceblocker.com", "sub": "user-1",
                "key_sha256": hex::encode(Sha256::digest(b"secret-key")),
                "iat": now, "exp": now + 86400,
                "offline_until": now + 86400 + OFFLINE_GRACE_SECONDS,
                "features": REQUIRED_FEATURES
            })
            .to_string(),
        );
        let body = format!("{header}.{payload}");
        let signature = URL_SAFE_NO_PAD.encode(signing.sign(body.as_bytes()).to_bytes());
        (format!("{body}.{signature}"), pub_key)
    }

    #[test]
    fn signed_lease_is_bounded_and_api_key_bound() {
        let now = 1_800_000_000;
        let (token, public_key) = signed_fixture(now);
        let until = verify_lease(&token, "secret-key", now, &public_key).unwrap();
        assert_eq!(until, now + LEASE_SECONDS + OFFLINE_GRACE_SECONDS);
        assert!(verify_lease(&token, "another-key", now, &public_key).is_err());
        assert!(verify_lease(&token, "secret-key", until + 1, &public_key).is_err());
        assert!(verify_lease(&token, "secret-key", now - 301, &public_key).is_err());
        let tampered = token.replacen('.', "X.", 1);
        assert!(verify_lease(&tampered, "secret-key", now, &public_key).is_err());
    }
}
