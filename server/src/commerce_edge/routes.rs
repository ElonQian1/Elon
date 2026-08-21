use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use anyhow::{bail, Result};
use axum::http::{uri::Authority, Method, Uri};

use super::config::EdgeConfig;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedRoute {
    instance_id: String,
    upstream_addr: SocketAddr,
    upstream_path: &'static str,
}

#[derive(Clone, Debug)]
struct RouteEntry {
    method: Method,
    target: ResolvedRoute,
}

#[derive(Clone, Debug)]
pub(crate) struct RouteTable {
    public_hosts: HashSet<String>,
    entries: HashMap<String, RouteEntry>,
    enabled_routes: usize,
}

pub(crate) struct RouteRegistry {
    current: RwLock<Arc<RouteTable>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RouteRejection {
    HostMissing,
    HostForbidden,
    QueryForbidden,
    NotFound,
    MethodNotAllowed,
}

impl RouteTable {
    pub(crate) fn from_config(config: &EdgeConfig) -> Result<Self> {
        let public_hosts = config.public_hosts().iter().cloned().collect();
        let mut entries = HashMap::new();
        let mut enabled_routes = 0usize;
        for route in config.routes().iter().filter(|route| route.enabled()) {
            enabled_routes += 1;
            insert_route(&mut entries, route, Method::GET, "/health")?;
            insert_route(&mut entries, route, Method::GET, "/commerce/v1/manifest")?;
            insert_route(&mut entries, route, Method::POST, "/commerce/v1/invoke")?;
        }
        if enabled_routes == 0 {
            bail!("COMMERCE_EDGE_ENABLED_ROUTE_MISSING");
        }
        Ok(Self {
            public_hosts,
            entries,
            enabled_routes,
        })
    }

    pub(crate) fn resolve(
        &self,
        host_header: Option<&str>,
        method: &Method,
        uri: &Uri,
    ) -> std::result::Result<ResolvedRoute, RouteRejection> {
        if uri.query().is_some() {
            return Err(RouteRejection::QueryForbidden);
        }
        self.validate_host(host_header)?;
        let entry = self
            .entries
            .get(uri.path())
            .ok_or(RouteRejection::NotFound)?;
        if &entry.method != method {
            return Err(RouteRejection::MethodNotAllowed);
        }
        Ok(entry.target.clone())
    }

    pub(crate) fn validate_host(
        &self,
        host_header: Option<&str>,
    ) -> std::result::Result<(), RouteRejection> {
        let host = normalize_request_host(host_header.ok_or(RouteRejection::HostMissing)?)
            .ok_or(RouteRejection::HostForbidden)?;
        if !self.public_hosts.contains(&host) {
            return Err(RouteRejection::HostForbidden);
        }
        Ok(())
    }

    pub(crate) fn enabled_routes(&self) -> usize {
        self.enabled_routes
    }
}

impl ResolvedRoute {
    pub(crate) fn instance_id(&self) -> &str {
        &self.instance_id
    }

    pub(crate) fn upstream_url(&self) -> String {
        format!("http://{}{}", self.upstream_addr, self.upstream_path)
    }
}

impl RouteRegistry {
    pub(crate) fn new(initial: RouteTable) -> Self {
        Self {
            current: RwLock::new(Arc::new(initial)),
        }
    }

    pub(crate) fn snapshot(&self) -> Arc<RouteTable> {
        self.current
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub(crate) fn replace(&self, candidate: RouteTable) {
        *self
            .current
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Arc::new(candidate);
    }
}

fn insert_route(
    entries: &mut HashMap<String, RouteEntry>,
    route: &super::config::MerchantRouteConfig,
    method: Method,
    upstream_path: &'static str,
) -> Result<()> {
    let public_path = format!("{}{}", route.public_base_path(), upstream_path);
    let entry = RouteEntry {
        method,
        target: ResolvedRoute {
            instance_id: route.instance_id().to_string(),
            upstream_addr: route.upstream_addr(),
            upstream_path,
        },
    };
    if entries.insert(public_path, entry).is_some() {
        bail!("COMMERCE_EDGE_ROUTE_DUPLICATE");
    }
    Ok(())
}

fn normalize_request_host(value: &str) -> Option<String> {
    value
        .parse::<Authority>()
        .ok()
        .map(|authority| authority.host().to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commerce_edge::config::EdgeConfig;

    fn table() -> RouteTable {
        let temp = std::env::temp_dir();
        let cert = temp
            .join("edge-cert.pem")
            .to_string_lossy()
            .replace('\\', "\\\\");
        let key = temp
            .join("edge-key.pem")
            .to_string_lossy()
            .replace('\\', "\\\\");
        let config = format!(
            r#"{{"schema":"yilong.commerce-edge.v1","listen_addr":"127.0.0.1:18443","certificate_chain_path":"{cert}","private_key_path":"{key}","public_hosts":["commerce.example.com"],"routes":[{{"instance_id":"coffee-a","public_base_path":"/merchants/coffee-a","upstream_addr":"127.0.0.1:18081"}}]}}"#
        );
        RouteTable::from_config(&EdgeConfig::parse(config.as_bytes()).unwrap()).unwrap()
    }

    #[test]
    fn route_requires_exact_host_method_and_path() {
        let table = table();
        let uri: Uri = "/merchants/coffee-a/commerce/v1/invoke".parse().unwrap();
        let target = table
            .resolve(Some("commerce.example.com:443"), &Method::POST, &uri)
            .unwrap();
        assert_eq!(target.instance_id(), "coffee-a");
        assert_eq!(
            target.upstream_url(),
            "http://127.0.0.1:18081/commerce/v1/invoke"
        );
        assert_eq!(
            table.resolve(Some("other.example.com"), &Method::POST, &uri),
            Err(RouteRejection::HostForbidden)
        );
        assert_eq!(
            table.resolve(Some("commerce.example.com"), &Method::GET, &uri),
            Err(RouteRejection::MethodNotAllowed)
        );
    }

    #[test]
    fn route_rejects_queries_and_management_paths() {
        let table = table();
        let query: Uri = "/merchants/coffee-a/health?verbose=1".parse().unwrap();
        assert_eq!(
            table.resolve(Some("commerce.example.com"), &Method::GET, &query),
            Err(RouteRejection::QueryForbidden)
        );
        let admin: Uri = "/merchants/coffee-a/api/admin/stores".parse().unwrap();
        assert_eq!(
            table.resolve(Some("commerce.example.com"), &Method::GET, &admin),
            Err(RouteRejection::NotFound)
        );
    }
}
