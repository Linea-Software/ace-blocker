# ACE Pro entitlement architecture

## Sources of truth

- **DodoPayments** sends signed subscription lifecycle webhooks to the ACE backend. It intentionally remains in `test_mode` even in release builds until the product changes its billing policy.
- **Better Auth** authenticates users and desktop API keys. The backend checks the effective Pro expiration before serving cloud backups, scripts or premium pack download URLs.
- **ACE backend** signs short-lived Pro entitlements using a server-only Ed25519 private key.
- **ACE Engine** verifies the signature using the server-delivered public key authenticated by HTTPS in production (loopback HTTP in dev) and enforces Pro independent of Svelte UI state.

## Signed entitlement protocol

`GET /api/v1/account/entitlement` accepts a desktop API key, authenticates that key independently of browser cookies and sets `Cache-Control: no-store`. It returns `{ "token": null, "publicKey": "<base64>" }` for Free accounts or a compact EdDSA-signed token with the same `publicKey` for active Pro. The public key is the 32-byte Ed25519 verification key, not the private signing key.

The token includes `iss = aceblocker.com`, the user ID (`sub`), SHA-256 fingerprint of the API key (`key_sha256`), `iat`, `exp`, `offline_until`, grants for Pro features, and `kid = ace-v1`. `exp` is at most 24 hours after issue and never later than the paid billing deadline. `offline_until` is exactly 72 hours after `exp`.

The Engine uses `ed25519-dalek 3.0` with strict verification and validates issuer, API-key binding, time bounds, offline deadline and features. Local booleans and user-editable backup metadata cannot establish Pro. Service-owned SQLite persists the signed lease, its authenticated public verification key and the last verified clock high-watermark; cached Pro is still computed from strict signature validation after restart.

## Refresh, expiry and revocation

The Engine refreshes after login and checkout return, on startup, and periodically while authenticated. Transient outages retain **only an unexpired signed entitlement** within the bounded offline window. Free, invalid-key and explicit-logout responses revoke Pro. Significant backward wall-clock changes invalidate locally cached Pro.

The proxy uses the verified, time-bounded local snapshot instead of waiting for a network request on each intercepted response. Previously configured network blocking rules and downloaded site packs remain active after Pro expires, rather than weakening protection. Creating or reconnecting paid resources, restoring custom packs, and cloud features require current entitlement. Server-origin premium data remains independently protected by server-side authorization.

## Payment integrity and deployment

The backend rejects duplicate and stale signed webhook events. A failed entitlement database write releases its event claim and propagates the error so the payment provider can retry. The server uses `ACE_ENTITLEMENT_SIGNING_KEY_BASE64` (private PKCS#8 DER, standard base64) and validates the matching `ACE_ENTITLEMENT_PUBLIC_KEY_BASE64`. Desktop binaries obtain the **public** key with the authenticated entitlement response at runtime, in both dev and release. TLS authentication protects initial key distribution in release. Local DB write protection protects the persisted verifier; compromise of that privileged storage or a trusted server response would weaken this trust-on-first-retrieval model. See `docs/RELEASING.md` and `ace-blocker-site/docs/DEPLOY.md` for generation and rotation.

No machine fingerprint or device quotas are enforced. A technically capable local administrator could modify the local binary; no fully local entitlement check guarantees perfect resistance to such tampering. The server controls all backend-hosted resources and ensures ordinary offline access is finite.

## Required regression cases

1. Invalid, Free, revoked, or another user's API key cannot receive Pro.
2. Signature alteration, key swapping, missing feature grants, expiration and clock rollback all deny local Pro.
3. Logging out or changing account clears the previous cached lease immediately.
4. Forged `requiresPro: false`, remote IDs, imported backup attachments, and premium resync requests cannot create unpaid capabilities.
5. Failed webhook writes are retryable; duplicate and stale events do not reactivate cancelled entitlements.
