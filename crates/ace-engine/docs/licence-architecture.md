# ACE Pro entitlement architecture

## Sources of truth

- **DodoPayments** sends signed subscription lifecycle webhooks to the ACE backend. Keep `test_mode` in release builds until the billing policy changes.
- **Better Auth** authenticates users and desktop API keys. The backend checks Pro expiration before serving cloud backups, scripts, or premium pack download URLs.
- **ACE backend** signs short-lived Pro entitlements using a server-only Ed25519 private key.
- **ACE Engine** verifies signatures with the server-delivered public key. HTTPS authenticates key delivery in production; development uses loopback HTTP. The engine enforces Pro without dependence on Svelte UI state.

## Signed entitlement protocol

`GET /api/v1/account/entitlement` accepts a desktop API key, authenticates that key independently of browser cookies and sets `Cache-Control: no-store`. It returns `{ "token": null, "publicKey": "<base64>" }` for Free accounts or a compact EdDSA-signed token with the same `publicKey` for active Pro. The public key is the 32-byte Ed25519 verification key, not the private signing key.

The token contains `iss = aceblocker.com`, the user ID (`sub`), SHA-256 fingerprint of the API key (`key_sha256`), `iat`, `exp`, `offline_until`, grants for Pro features, and `kid = ace-v1`. `exp` is at most 24 hours after issue and never later than the paid billing deadline. `offline_until` is exactly 72 hours after `exp`.

The engine uses `ed25519-dalek 3.0` for strict verification. It checks the issuer, API-key binding, time bounds, offline deadline, and feature grants. Local booleans and user-editable backup metadata cannot establish Pro. Service-owned SQLite stores the signed lease, its authenticated public verification key, and the last verified clock high-watermark. After restart, strict signature validation determines cached Pro.

## Refresh, expiry and revocation

The engine refreshes after login and checkout return. It also refreshes on startup and periodically while authenticated. Transient outages retain **only an unexpired signed entitlement** within the bounded offline window. Free, invalid-key and explicit-logout responses revoke Pro. Significant backward wall-clock changes invalidate cached Pro.

The proxy uses a verified, time-bounded local snapshot. It does not wait for a network request on each intercepted response. Previously configured network blocking rules and downloaded site packs remain active after Pro expires, rather than weakening protection. Creating or reconnecting paid resources, restoring custom packs, and cloud features require current entitlement. Server-origin premium data remains independently protected by server-side authorization.

## Payment integrity and deployment

The backend rejects duplicate and stale signed webhook events. If an entitlement database write fails, release its event claim and propagate the error. This lets the payment provider retry. The server uses `ACE_ENTITLEMENT_SIGNING_KEY_BASE64` (private PKCS#8 DER, standard base64) and validates the matching `ACE_ENTITLEMENT_PUBLIC_KEY_BASE64`. Desktop binaries obtain the **public** key with the authenticated entitlement response at runtime, in both dev and release. TLS authentication protects initial key distribution in release. Local DB write protection protects the persisted verifier; compromise of that privileged storage or a trusted server response would weaken this trust-on-first-retrieval model. See `docs/RELEASING.md` and `ace-blocker-site/docs/DEPLOY.md` for generation and rotation.

The system does not enforce machine fingerprints or device quotas. A technically capable local administrator can modify the local binary. No fully local entitlement check can prevent all such tampering. The server controls all backend-hosted resources and ensures ordinary offline access is finite.

## Required regression cases

1. Invalid, Free, revoked, or another user's API key cannot receive Pro.
2. Signature alteration, key swapping, missing feature grants, expiration and clock rollback all deny local Pro.
3. Logging out or changing account clears the previous cached lease immediately.
4. Forged `requiresPro: false`, remote IDs, imported backup attachments, and premium resync requests cannot create unpaid capabilities.
5. Failed webhook writes are retryable; duplicate and stale events do not reactivate canceled entitlements.
