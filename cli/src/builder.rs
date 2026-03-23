use crate::cli::ProxyMode;
use anyhow::Context;
use reqwest::Url;
use reqwest::blocking::Client;
use std::net::IpAddr;
use uuid::Uuid;

#[derive(Debug)]
pub struct ReqClientBuilder {
    pub session_id: String,
    pub base_url: String,
    pub token: Option<String>,
    pub timeout_ms: u64,
    pub proxy_mode: ProxyMode,
}

impl ReqClientBuilder {
    pub fn new(base_url: String, timeout_ms: u64, proxy_mode: ProxyMode) -> Self {
        let session_id = format!("session-{}", Uuid::new_v4());
        ReqClientBuilder {
            session_id,
            base_url,
            token: None,
            timeout_ms,
            proxy_mode,
        }
    }

    pub fn with_token(mut self, token: Option<String>) -> Self {
        self.token = token;
        self
    }

    pub fn build(&self) -> anyhow::Result<Client> {
        let builder = Client::builder().timeout(std::time::Duration::from_millis(self.timeout_ms));
        let result = match self.proxy_mode {
            ProxyMode::System => builder.build(),
            ProxyMode::Direct => builder.no_proxy().build(),
            ProxyMode::Auto => {
                let proxy_mode = Self::effective_proxy(self, &self.base_url);
                match proxy_mode {
                    ProxyMode::System => builder.build(),
                    ProxyMode::Direct => builder.no_proxy().build(),
                    ProxyMode::Auto => unreachable!(),
                }
            }
        };
        result.with_context(|| "build http client failed")
    }

    fn effective_proxy(runtime: &ReqClientBuilder, request_url: &str) -> ProxyMode {
        match runtime.proxy_mode {
            ProxyMode::Direct => ProxyMode::Direct,
            ProxyMode::System => ProxyMode::System,
            ProxyMode::Auto => {
                if should_bypass_proxy(request_url) {
                    ProxyMode::Direct
                } else {
                    ProxyMode::System
                }
            }
        }
    }
}

pub fn should_bypass_proxy(request_url: &str) -> bool {
    let Ok(url) = Url::parse(request_url) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        Ok(IpAddr::V6(ip)) => {
            ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local()
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::should_bypass_proxy;

    #[test]
    fn bypasses_local_and_private_targets() {
        assert!(should_bypass_proxy("http://127.0.0.1:8081/health"));
        assert!(should_bypass_proxy("http://localhost:8081/health"));
        assert!(should_bypass_proxy("http://192.168.50.214:9998/health"));
    }

    #[test]
    fn does_not_bypass_public_targets() {
        assert!(!should_bypass_proxy("http://8.8.8.8:80/health"));
        assert!(!should_bypass_proxy("https://example.com/health"));
    }
}
