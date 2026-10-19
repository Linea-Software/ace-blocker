//! The interception decision point.
//!
//! Before a `CONNECT` tunnel is TLS-terminated the proxy asks the registered
//! [`InterceptDecider`] whether that host should be MITM'd at all. Tunnels the
//! decider rejects are relayed to the origin byte-for-byte with no TLS setup:
//! no leaf certificate is minted and no decryption occurs. This is what keeps
//! interception *narrow* — only hosts explicitly opted in are ever decrypted.

use http::uri::Authority;

/// Decides whether a `CONNECT` tunnel should be MITM-terminated.
///
/// Returning `false` sends the tunnel through an opaque byte-for-byte relay
/// to the origin: no leaf certificate is minted and no decryption occurs.
///
/// `sni` supplies the ClientHello name when transparent IP CONNECT recovers it.
/// The original ClientHello bytes are replayed unchanged.
/// Explicit forward-proxy connections may still supply `None`.
/// Only the opted-in transparent transport peeks at TLS SNI.
pub trait InterceptDecider: Send + Sync {
    /// Return `true` to terminate TLS and MITM this tunnel, `false` to relay
    /// it untouched.
    fn should_intercept(&self, authority: &Authority, sni: Option<&str>) -> bool;

    /// Opt in to reading TLS SNI when a transparent CONNECT has only an IP.
    /// The default stays false for ordinary explicit forward proxies.
    fn recover_hostname_for_ip_connect(&self) -> bool {
        false
    }
}

/// Default [`InterceptDecider`]: intercept nothing.
///
/// Without an explicitly registered decider the proxy never decrypts by
/// default — every tunnel is relayed opaquely (fail-closed / fail-open-to-
/// tunnel), so no leaf certificate is ever minted for a host that was not
/// opted into interception.
pub struct NoInterceptDecider;

impl InterceptDecider for NoInterceptDecider {
    fn should_intercept(&self, _authority: &Authority, _sni: Option<&str>) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_intercept_decider_never_intercepts() {
        let decider = NoInterceptDecider;
        let authority: Authority = "example.com:443".parse().unwrap();
        assert!(!decider.should_intercept(&authority, None));
        assert!(!decider.should_intercept(&authority, Some("example.com")));
    }
}
