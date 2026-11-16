//! TLS client config for `https://` upstream proxies (rustls, webpki roots, aws-lc-rs).

use rustls::{ClientConfig, RootCertStore};
use std::sync::Arc;

pub(crate) fn config_with_roots(roots: RootCertStore) -> Arc<ClientConfig> {
    let cfg = ClientConfig::builder_with_provider(Arc::new(rustls::crypto::aws_lc_rs::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("aws-lc-rs supports the default TLS versions")
        .with_root_certificates(roots)
        .with_no_client_auth();
    Arc::new(cfg)
}

/// Production config: the public web PKI roots.
pub(crate) fn default_config() -> Arc<ClientConfig> {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    config_with_roots(roots)
}
