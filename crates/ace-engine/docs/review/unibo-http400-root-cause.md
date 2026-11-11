# UniBo HTTP 400 investigation — September 23, 2026

## Finding

When the proxy forwards HTTP/2 as HTTP/1.1, it does not create the required
`Host` header from `:authority`. The browser sends a valid request. The proxy
then sends invalid HTTP/1.1. The UniBo nginx server rejects this request
with status 400.

The investigation used source revision `6fcc7cd` and the repository's
`mitm-proxy` crate. The first investigation did not change production behavior.
The Repair section describes the later fix.

## Evidence

Target: `https://www.unibo.it/it/studiare/vivere-luniversita-e-la-citta/wi-fi/faq-almawifi`.
The live origin resolved to `137.204.24.208`. When the client offered
`h2` and `http/1.1`, the origin selected `http/1.1`.

| Experiment | Result |
| --- | --- |
| Direct curl request | 200 OK |
| Direct HTTP/1.1 request with `Host` omitted | 400 Bad Request, nginx |
| HTTP/1.1 client through the repository proxy | 200 OK, 474573 body bytes |
| HTTP/2 client through the same proxy | 400 Bad Request, 150 body bytes |
| Same HTTP/2 request with a test-only inspector supplying `Host` from the URI authority | 200 OK, 474573 body bytes |

The proxy tests send all reported headers, including the Brave search referrer.
They do not send cookies or credentials. The test inspector changes only
`Host`. The test helpers disable upstream certificate verification for local,
self-signed origins. The direct curl control verified the UniBo certificate.
The tests reproduce the issue with the embedded proxy. They do not capture
the user's active Brave connection, engine connection, or policy state. The
live error contained the same nginx HTML core as the reported response. It
did not contain the optional browser-padding comments.

A local TLS origin advertising only HTTP/1.1 captured the proxy's raw output:

```http
GET https://localhost:46223/it/studiare/vivere-luniversita-e-la-citta/wi-fi/faq-almawifi HTTP/1.1
accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8
accept-language: en-US,en;q=0.8
cache-control: max-age=0
priority: u=0, i
upgrade-insecure-requests: 1
referer: https://safe.search.brave.com/

```

The request has no `Host` header. The absolute request target still contains
the URI authority. HTTP/1.1 still requires `Host`. A direct curl control
sent an absolute request target with a valid `Host` header. It returned 200.
This result isolates the missing header as the cause of status 400.

## Code path and underlying design error

1. `crates/mitm-proxy/src/http.rs::mitm_tunnel` negotiates the browser-facing
   protocol independently of the origin-facing protocol. HTTP/2 is available.
2. `relay` preserves the incoming request version and authority and calls
   `util::ensure_host_header` before the upstream protocol has been selected.
3. `crates/mitm-proxy/src/util.rs::ensure_host_header` returns immediately for
   HTTP/2 and HTTP/3. This uses the **incoming** protocol to decide an
   **outgoing** protocol requirement.
4. `crates/mitm-proxy/src/upstream.rs::send_secure_reusable_h2` negotiates
   HTTP/1.1 with UniBo and calls `send_h1` with the request unchanged.
5. `send_h1` uses Hyper's low-level connection API. Hyper's HTTP/1 encoder
   coerces the HTTP/2 request version to HTTP/1.1 on the wire but does not
   manufacture a `Host` header. The raw capture confirms this behavior.

The proxy must normalize a request for the upstream protocol that it selects.
This error can also affect intercepted traffic to other HTTP/1.1 origins.
Opaque CONNECT tunnels do not perform this conversion.

RFC references:

- [RFC 9113 §8.3.1](https://www.rfc-editor.org/rfc/rfc9113.html#section-8.3.1)
  requires an intermediary converting to HTTP/1.1 to create `Host` from
  `:authority` when the latter is present.
- [RFC 9112 §3.2](https://www.rfc-editor.org/rfc/rfc9112.html#section-3.2)
  requires HTTP/1.1 `Host`, and 400 for a request without it.

## Why existing tests missed it

`tests/common/mod.rs::proxy_connect_get` explicitly always sends HTTP/1.1 and
supplies `Host`, including when called from `proxy_http2.rs`. The shared HTTPS
origin supports HTTP/2, and several actual HTTP/2 test requests explicitly
include `Host`. These tests do not cover a browser-like HTTP/2 request with only an authority
header when the upstream origin supports only HTTP/1.1. Unit tests in
`util.rs` also verify that `ensure_host_header` skips HTTP/2. They do not
test the later conversion to HTTP/1.1.

## Reproduce

Before the repair, the two diagnostic tests failed. One detected a missing
`Host` header. The other detected an incorrect HTTP/2 status. The local
wire-capture test now runs as a regular regression test. The live test is
ignored by default because it contacts a public server. It retains the
original test-only `Host` control.

```sh
# Local wire capture; needs permission to bind loopback sockets.
cargo test -p mitm-proxy --test unibo_diagnosis http2_to_http1_preserves_authority_as_host -- --nocapture

# Live three-way comparison; makes three unauthenticated GETs to UniBo.
cargo test -p mitm-proxy --test unibo_diagnosis unibo_http2_and_http1_both_succeed -- --ignored --nocapture
```

## Repair

`upstream.rs::send_h1` now prepares a request after it selects HTTP/1. It
creates `Host` from the effective URI authority. It replaces conflicting or
duplicate values. It converts HTTP/2 and HTTP/3 versions to HTTP/1.1.
An origin-form request keeps its existing `Host` header. A request without
an authority or `Host` returns an error. The proxy does not send invalid
HTTP for that request. The fix preserves existing HTTP/1 versions, upgrade
headers, request targets, and bodies. New TLS connections, cached HTTP/1
origin connections, and plaintext upstream requests use this path.

The local proxy test now runs by default. Additional wire-level tests cover
HTTP/2 and HTTP/3 conversion. They cover conflicting or duplicate `Host`
values, default and non-default ports, and bracketed IPv6 authorities. Other
tests check preservation of HTTP/1 upgrades and handling of HTTP/1.0
origin-form requests. The fix applies to all sites. It adds no site-specific
exception or caller workaround.

After the repair, the live UniBo comparison returned 200 for HTTP/1.1,
HTTP/2 without the test inspector, and HTTP/2 with the inspector. Each
response contained 474573 body bytes. `cargo test -p mitm-proxy` passed
100 unit and integration tests and one doctest. The live check was the only
test ignored by default. The separate live check passed.
`cargo clippy -p mitm-proxy --all-targets` completed. It reported existing
warnings in unchanged code.
