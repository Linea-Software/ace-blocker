//! Dynamic per-host certificate resolution for rustls.
//!
//! [`DynamicCertResolver`] implements [`rustls::server::ResolvesServerCert`].
//! During each TLS handshake rustls calls [`resolve`](DynamicCertResolver::resolve)
//! with the client's SNI; we mint (and cache) a leaf certificate for that host
//! signed by the proxy CA. The same resolver instance is shared by the TCP
//! (HTTP/1.x + h2) and QUIC (HTTP/3) listeners.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use rustls::crypto::ring::sign::any_supported_type;
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use tracing::{debug, warn};

use crate::ca::CertAuthority;

/// Generates leaf certificates on demand and caches them by host.
#[derive(Debug)]
pub struct DynamicCertResolver {
    ca: Arc<CertAuthority>,
    cache: Mutex<CertificateCache>,
}

const MAX_CACHED_LEAF_CERTIFICATES: usize = 512;

#[derive(Debug, Default)]
struct CertificateCache {
    entries: HashMap<String, Arc<CertifiedKey>>,
    insertion_order: VecDeque<String>,
}

impl CertificateCache {
    fn insert(&mut self, host: String, key: Arc<CertifiedKey>) {
        if self.entries.contains_key(&host) {
            self.entries.insert(host, key);
            return;
        }
        if self.entries.len() >= MAX_CACHED_LEAF_CERTIFICATES {
            if let Some(oldest) = self.insertion_order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
        self.insertion_order.push_back(host.clone());
        self.entries.insert(host, key);
    }
}

// `CertAuthority` holds an rcgen `Issuer`, which is not `Debug`; provide a
// minimal manual impl so the resolver can derive/participate in `Debug`.
impl std::fmt::Debug for CertAuthority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CertAuthority").finish_non_exhaustive()
    }
}

impl DynamicCertResolver {
    pub fn new(ca: Arc<CertAuthority>) -> Self {
        Self {
            ca,
            cache: Mutex::new(CertificateCache::default()),
        }
    }

    /// Get a cached certified key for `host`, or mint and cache a new one.
    pub(crate) fn certified_key(&self, host: &str) -> Option<Arc<CertifiedKey>> {
        if let Some(existing) = self.cache.lock().unwrap().entries.get(host) {
            return Some(existing.clone());
        }

        let (chain, key_der) = match self.ca.issue_leaf(host) {
            Ok(pair) => pair,
            Err(error) => {
                warn!("failed to issue leaf for {host}: {error:#}");
                return None;
            }
        };

        let signing_key = match any_supported_type(&key_der) {
            Ok(k) => k,
            Err(error) => {
                warn!("unsupported leaf key type for {host}: {error}");
                return None;
            }
        };

        let certified = Arc::new(CertifiedKey::new(chain, signing_key));
        debug!("issued leaf certificate for {host}");
        self.cache
            .lock()
            .unwrap()
            .insert(host.to_string(), certified.clone());
        Some(certified)
    }

    /// Expose the cache size for testing.
    #[cfg(test)]
    pub(crate) fn cache_len(&self) -> usize {
        self.cache.lock().unwrap().entries.len()
    }

    /// Whether a leaf certificate is cached for `host`. Test-only: used to
    /// assert that tunneled (non-intercepted) hosts never get a cert minted.
    #[cfg(test)]
    pub(crate) fn has_cached_key(&self, host: &str) -> bool {
        self.cache.lock().unwrap().entries.contains_key(host)
    }
}

impl ResolvesServerCert for DynamicCertResolver {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let host = client_hello.server_name().unwrap_or("localhost").to_owned();
        self.certified_key(&host)
    }
}

/// QUIC-only gate: check SNI before consulting the shared certificate factory.
#[cfg(feature = "http3")]
pub(crate) struct Http3CertResolver {
    inner: Arc<DynamicCertResolver>,
    policy: Arc<dyn Fn(&str) -> bool + Send + Sync>,
}

#[cfg(feature = "http3")]
impl Http3CertResolver {
    pub(crate) fn new(
        inner: Arc<DynamicCertResolver>,
        policy: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    ) -> Self {
        Self { inner, policy }
    }
}

#[cfg(feature = "http3")]
impl std::fmt::Debug for Http3CertResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Http3CertResolver").finish_non_exhaustive()
    }
}

#[cfg(feature = "http3")]
impl ResolvesServerCert for Http3CertResolver {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let host = client_hello.server_name()?;
        if !(self.policy)(host) {
            return None;
        }
        self.inner.certified_key(host)
    }
}

/// Per-CONNECT resolver that uses the tunnel authority when the ClientHello has
/// no SNI. IP-address TLS clients do not send SNI, but rustls can still verify an
/// IP-SAN certificate, so the CONNECT target is the correct fallback identity.
#[derive(Debug)]
pub(crate) struct TunnelCertResolver {
    inner: Arc<DynamicCertResolver>,
    fallback_host: String,
}

impl TunnelCertResolver {
    pub(crate) fn new(inner: Arc<DynamicCertResolver>, fallback_host: String) -> Self {
        Self {
            inner,
            fallback_host,
        }
    }
}

impl ResolvesServerCert for TunnelCertResolver {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let host = client_hello.server_name().unwrap_or(&self.fallback_host);
        self.inner.certified_key(host)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ca::CertAuthority;

    fn make_ca() -> Arc<CertAuthority> {
        let (cert_pem, key_pem) = CertAuthority::generate_root().unwrap();
        Arc::new(CertAuthority::from_pem(cert_pem, &key_pem).unwrap())
    }

    #[test]
    fn resolve_returns_cert_for_sni() {
        let ca = make_ca();
        let resolver = DynamicCertResolver::new(ca);

        // Use the internal method directly
        let cert = resolver.certified_key("example.com");
        assert!(cert.is_some(), "should return a cert for valid host");
    }

    #[test]
    fn resolve_caches_same_arc_on_repeat() {
        let ca = make_ca();
        let resolver = DynamicCertResolver::new(ca);

        let c1 = resolver.certified_key("example.com").unwrap();
        assert_eq!(resolver.cache_len(), 1);

        let c2 = resolver.certified_key("example.com").unwrap();
        assert_eq!(resolver.cache_len(), 1, "cache should not grow on repeat");

        // Same Arc (pointer identity)
        assert!(Arc::ptr_eq(&c1, &c2), "must return the same cached Arc");
    }

    #[test]
    fn resolve_ip_sni_works() {
        let ca = make_ca();
        let resolver = DynamicCertResolver::new(ca);
        let cert = resolver.certified_key("127.0.0.1");
        assert!(cert.is_some(), "should handle IP SNI");
    }

    #[test]
    fn resolve_different_hosts_separate_cache() {
        let ca = make_ca();
        let resolver = DynamicCertResolver::new(ca);

        let _a = resolver.certified_key("foo.example.com").unwrap();
        let _b = resolver.certified_key("bar.example.com").unwrap();
        assert_eq!(
            resolver.cache_len(),
            2,
            "different hosts = separate entries"
        );
    }
    #[test]
    fn leaf_certificate_cache_evicts_oldest_entries() {
        let resolver = DynamicCertResolver::new(make_ca());
        let key = resolver.certified_key("first.example").unwrap();
        let mut cache = resolver.cache.lock().unwrap();
        for n in 0..MAX_CACHED_LEAF_CERTIFICATES {
            cache.insert(format!("host-{n}.example"), key.clone());
        }
        assert_eq!(cache.entries.len(), MAX_CACHED_LEAF_CERTIFICATES);
        assert!(!cache.entries.contains_key("first.example"));
        assert!(cache.entries.contains_key("host-0.example"));
    }
}
