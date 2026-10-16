# License Management Architecture

**Integration: Ace Engine <-> Custom Backend <-> Polar.sh**

## 1. Overview

The goal is to implement a robust license management system for **Ace Engine** using **Polar.sh** as the Merchant/Subscription provider, mediated by a **Custom Backend Service**.

This "Backend-Mediated" approach offers several advantages over communicating directly with Polar from the desktop client:

- **Security**: Sensitive API keys or logic remain on the server.
- **Control**: We can implement custom logic (node locking, grace periods, usage tracking) independent of Polar's constraints.
- **Decoupling**: If we switch providers later, the Ace Engine contract remains the same.
- **User Experience**: Users manage their account and subscription on the official website, and authentication is seamlessly transferred to the desktop app via a custom URL scheme.

## 2. System Components

### A. Polar.sh (Merchant of Record)

- **Role**: Handles checkout, subscription billing, and acts as the "Source of Truth" for license validity.
- **Actions**:
  - Issues License Keys upon purchase.
  - Sends **Webhooks** (checkout.completed, subscription.updated, subscription.deleted) to the Custom Backend.

### B. Custom Backend Service

- **Role**: The central authority for Ace Engine instances.
- **Database**:
  - `licenses`: { key, status, polar_sub_id, max_seats, current_seats, last_validated_at }
  - `instances`: { machine_id, license_key, checks_count, last_ip }
- **API Endpoints**:
  - `POST /v1/license/activate`: Validates a key/token and binds it to a `machine_id`.
  - `POST /v1/license/validate`: Periodic heartbeats from Ace Engine.
  - `POST /webhooks/polar`: Receives updates from Polar.sh.
  - `GET /v1/auth/callback`: Handles OAuth or Magic Link callbacks (if applicable) and redirects to `aceblocker://`.

### C. Ace Engine (Local Desktop Service)

- **Role**: Enforces the license locally.
- **Storage**: Stores the `license_key` and a signed `license_token` (or status+expiry) in the local encrypted database (`engine.db`).
- **Logic**:
  - **Login/Activation**: Opens Website -> Website triggers `aceblocker://` -> Validates Token with Backend.
  - **Enforcement**: Middleware (`RequiresProSubscription`) checks functionality access.
  - **Background Sync**: Periodically validates with Backend to handle cancellations/expirations.

---

## 3. Detailed Workflows

### 3.1. Purchase & Login Flow (Engine -> Website -> Engine)

1.  **User** clicks "Login / Upgrade" in the Ace Engine UI.
2.  **Ace Engine** opens the official project website (e.g., `https://aceblocker.com/login`) in the default system browser.
3.  **User** on the Website:
    - Logs in or registers an account.
    - Purchases a Pro subscription (handled via Polar.sh embedded checkout or redirect).
4.  **Website** (upon successful login/purchase):
    - Redirects the browser to `aceblocker://login?token=<secure_session_token>` (or `key=<license_key>`).
5.  **OS** prompts the user to open "Ace Engine".
6.  **Ace Engine** receives the deep link event.

### 3.2. Activation (Engine -> Backend)

1.  **Ace Engine** parses the `aceblocker://` URL to extract the `token` or `key`.
2.  **Ace Engine** generates a stable `machine_id` (e.g., hashed HWID).
3.  **Ace Engine** calls the **Custom Backend**:
    - `POST /v1/license/activate { token, machine_id }`.
4.  **Custom Backend**:
    - Validates the token/key.
    - Checks seat limits (Node Locking).
    - Binds `machine_id` to the license.
    - Returns `{ valid: true, license_key: "...", expires_at: "...", signature: "..." }`.
5.  **Ace Engine** stores the result in `Settings`:
    - Saves the `license_key` for future validations.
    - Enables Pro features.

### 3.3. Runtime Enforcement & Offline Grace

1.  **Ace Engine** holds a `ProStatus { is_active, expires_at, last_check }`.
2.  **Middleware** matches:
    ```rust
    if self.is_pro() {
        // Allow
    } else {
        // Reject gRPC call
    }
    ```
3.  **Offline Strategy**:
    - If `expires_at` is in the future, allow access even without internet.
    - If `expires_at` has passed + **Grace Period** (e.g., 3 days), downgrade to Free.

### 3.4. Validation / Heartbeat

1.  Every 24 hours (or on startup), **Ace Engine** attempts `POST /v1/license/validate` using the stored `license_key`.
2.  **Backend** checks Polar (or local cached state) to see if subscription is still active.
3.  If `active`, Backend returns refreshed `expires_at`.
4.  If `canceled/past_due`, Backend returns `valid: false`.
5.  **Ace Engine** updates local state.

---

## 4. Implementation Plan

### Phase 1: Engine Modifications

1.  **URL Scheme Registration**:
    - Ensure the installer supports registering the `aceblocker://` custom protocol on Windows (Registry) and macOS/Linux.
2.  **Deep Link Handling**:
    - Implement a listener in the main application loop to catch `aceblocker://` events.
    - Pass the captured URL to the `LicenceManager`.
3.  **Update `LicenceManager` (`src/manager/licence.rs`)**:
    - Remove direct "Enter Key" logic if it's strictly web-based (or keep as fallback).
    - Add `handle_deep_link(url)` method.
    - Update `activate_license` to accept the token/key from the URL.
    - Implement `open_login_page`: simply opens the URL.
4.  **Update `EngineApi`**:
    - `Login()` gRPC method: Trigger opening the browser.

### Phase 2: Web & Backend Development

1.  **Website**:
    - Implement Auth/Purchase flow.
    - Add logic to redirect to `aceblocker://login` after success.
2.  **Backend**:
    - Endpoint to exchange the "web token" for a "desktop license session".

### Phase 3: Integration

1.  Point Ace Engine to `https://aceblocker.com` (or staging).
2.  Test the full loop: App -> Browser -> Purchase -> App Deep Link -> Activation.

## 5. Security & Anti-Abuse

- **Node Locking**: Prevents one key from being used on 100 machines. Backend enforces `count(instances) <= max_seats`.
- **Validation Signature**: Backend signs the validation response. Engine verifies signature to prevent local proxy attacks (Man-in-the-Middle on localhost).
- **Time Tampering**: Engine uses `expires_at` (UTC). If local time is wildly inconsistent or backward, force online validation.
