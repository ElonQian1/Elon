//! HTTP clients for child-app service calls.

use std::{sync::OnceLock, time::Duration};

const FB2_CONNECT_TIMEOUT_SECS: u64 = 10;

pub(crate) fn fb2_direct_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        build_fb2_direct_client().expect("fb2 direct HTTP client should initialize")
    })
}

pub(crate) fn build_fb2_direct_client() -> Result<reqwest::Client, reqwest::Error> {
    fb2_direct_client_builder().build()
}

fn fb2_direct_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .no_proxy()
        .pool_max_idle_per_host(0)
        .connect_timeout(Duration::from_secs(FB2_CONNECT_TIMEOUT_SECS))
}
