# UniBo HTTP 400 investigation — 2026-09-23

## Finding

The intercepted HTTP/2 → HTTP/1.1 forwarding path fails to reconstruct the
mandatory `Host` header from the request's `:authority`. The request is valid
when the browser sends it, but the proxy emits invalid HTTP/1.1. UniBo's nginx
rejects it with 400.

Investigated against source revision `6fcc7cd`, using the in-repository
`mitm-proxy` crate. The initial investigation did not change production behavior;
the subsequent repair is described below.

## Evidence

Target: `https://www.unibo.it/it/studiare/vivere-luniversita-e-la-citta/wi-fi/faq-almawifi`.
The live origin resolved to `137.204.24.208` and negotiated `http/1.1` when
offered both `h2` and `http/1.1`.

| Experiment | Result |
| --- | --- |
| Direct curl request | 200 OK |
| Direct HTTP/1.1 request with `Host` omitted | 400 Bad Request, nginx |
| HTTP/1.1 client through the repository proxy | 200 OK, 474573 body bytes |
| HTTP/2 client through the same proxy | 400 Bad Request, 150 body bytes |
| Same HTTP/2 request with a test-only inspector supplying `Host` from the URI authority | 200 OK, 474573 body bytes |

The proxy tests send all headers supplied in the report, including the Brave
search referrer, with no cookies or credentials. The experimental inspector
changes only `Host`. Test helpers disable upstream certificate verification
for local self-signed origins; the direct curl baseline verified UniBo's
certificate successfully. This is a reproduction using the embedded proxy,
not a capture of the user's running Brave/engine connection or policy state.
The live error contained the same nginx HTML core as the reported response,
without its optional browser padding comments.

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

There is no `Host`. The URI authority remains in the absolute request target,
but that does not remove HTTP/1.1's requirement for the header. A separate
direct curl control using an absolute request target and a valid `Host`
returned 200, isolating the missing header as the cause of this 400.

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

The missing layer is request normalization for the protocol actually selected
on the upstream connection. This affects intercepted traffic to other
HTTP/1.1 origins too. Opaque CONNECT tunnels do not perform this conversion.

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
include `Host`. These do not establish the browser-like case of an authority-only
HTTP/2 request downgraded to an HTTP/1.1-only origin. Unit tests in `util.rs`
also assert that `ensure_host_header` skips HTTP/2 without testing the later
downgrade.

## Reproduce

Before the repair, the two diagnostic tests failed at the missing-Host and
HTTP/2-status assertions respectively. The local wire-capture test now runs
as an ordinary regression test. The live test remains ignored by default
because it contacts a public server. It also retains the test-only Host
control from the original investigation.

```sh
# Local wire capture; needs permission to bind loopback sockets.
cargo test -p mitm-proxy --test unibo_diagnosis http2_to_http1_preserves_authority_as_host -- --nocapture

# Live three-way comparison; makes three unauthenticated GETs to UniBo.
cargo test -p mitm-proxy --test unibo_diagnosis unibo_http2_and_http1_both_succeed -- --ignored --nocapture
```

## Repair

`upstream.rs::send_h1` now prepares the request after HTTP/1 has been selected.
It derives `Host` from the effective URI authority, replacing conflicting or
duplicate values, and explicitly converts HTTP/2 and HTTP/3 versions to
HTTP/1.1. An origin-form request retains its existing Host; a request with
neither authority nor Host returns an error instead of emitting invalid HTTP.
Existing HTTP/1 versions, upgrade headers, request targets, and bodies remain
intact. Both first-time TLS negotiation and cached HTTP/1 origin paths pass
through this boundary, as do plaintext upstream requests.

The local proxy reproduction now runs by default. Additional wire-level
tests exercise HTTP/2 and HTTP/3 downgrades, conflicting and duplicate Host
values, default and non-default ports, and bracketed IPv6 authorities. Other
tests check HTTP/1 upgrade preservation and HTTP/1.0 origin-form handling.
The repair applies generally; it adds no site exemption or caller workaround.

Post-repair validation: the live UniBo comparison returned 200 for HTTP/1.1,
HTTP/2 without the test inspector, and HTTP/2 with the inspector (474573 body
bytes each). `cargo test -p mitm-proxy` passed 100 unit/integration tests and
one doctest, with only the live check ignored by default. The separately run
live check passed. `cargo clippy -p mitm-proxy --all-targets` completed with
existing warnings in unchanged code.
