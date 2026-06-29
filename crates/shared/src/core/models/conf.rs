use std::collections::HashMap;

use config::CONF;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValue;

const DIRECT: &str = "DIRECT";
const CHECK_URL: &str = "https://www.gstatic.com/generate_204";
pub const DATA_URL: &str =
    "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/";

/// Mihomo configuration file
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Mihomo {
    #[serde(flatten)]
    pub net: Net,

    #[serde(rename = "geodata-mode")]
    pub geo_mode: bool,
    #[serde(rename = "geox-url")]
    pub geo_x: Geo,
    #[serde(rename = "geo-auto-update")]
    pub geo_update: bool,
    #[serde(rename = "geo-update-interval")]
    pub geo_update_interval: i32,

    profile: Profile,

    #[serde(rename = "external-controller")]
    pub external_controller: String,

    #[serde(rename = "log-level")]
    pub level: String,
    #[serde(rename = "find-process-mode")]
    pub find_process: String,

    pub secret: String,
    pub dns: Dns,
    pub tun: Tun,
    pub proxies: Vec<Proxy>,

    #[serde(rename = "proxy-groups")]
    pub groups: Vec<GroupProxy>,
    #[serde(rename = "proxy-providers")]
    pub providers: HashMap<String, Provider>,

    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Geo {
    pub geoip: String,
    pub geosite: String,
    pub mmdb: String,
    pub asn: String,
}

/// Network configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Net {
    #[serde(rename = "allow-lan")]
    pub allow_lan: bool,

    #[serde(rename = "bind-address")]
    bind_addr: String,

    #[serde(rename = "mixed-port")]
    pub mixed: u16,

    pub mode: String,
    pub ipv6: bool,

    #[serde(rename = "tcp-concurrent")]
    pub tcp_concurrent: bool,
}

/// DNS configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Dns {
    pub enable: bool,
    pub ipv6: bool,

    #[serde(rename = "enhanced-mode")]
    pub enhanced_mode: String,

    #[serde(rename = "fake-ip-range")]
    pub fake_range: String,
    #[serde(rename = "fake-ip-range6")]
    pub fake_range6: String,
    #[serde(rename = "fake-ip-filter-mode")]
    pub fake_filter_mode: String,
    #[serde(rename = "fake-ip-filter")]
    pub fake_filter: Vec<String>,

    pub listen: String,

    #[serde(rename = "proxy-server-nameserver")]
    pub proxy_ns: Vec<String>,
    #[serde(rename = "default-nameserver")]
    pub default_ns: Vec<String>,
    pub nameserver: Vec<String>,
    pub fallback: Vec<String>,
}

/// Tun Interface configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct Tun {
    pub enable: bool,
    pub device: String,
    pub stack: String,
    #[serde(rename = "auto-route")]
    pub route: bool,
    #[serde(rename = "auto-detect-interface")]
    pub detect: bool,
    #[serde(rename = "strict-route", default)]
    pub strict: bool,
    #[serde(rename = "dns-hijack")]
    pub dns: Vec<String>,
}

/// Simple Proxy configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Proxy {
    #[serde(rename = "type")]
    pub type_name: String,
    pub name: String,
    pub server: String,
    pub port: u16,

    pub uuid: Option<String>,
    pub udp: Option<bool>,
    pub tls: Option<bool>,

    #[serde(default)]
    pub servername: String,
    pub network: String,

    #[serde(rename = "reality-opts", skip_serializing_if = "Option::is_none")]
    pub reality: Option<Reality>,

    #[serde(rename = "client-fingerprint")]
    pub fingerprint: String,

    #[serde(flatten)]
    pub extra: HashMap<String, YamlValue>,
}

/// Reality properties for Simple Proxy
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Reality {
    #[serde(rename = "public-key")]
    pub public: String,
    #[serde(rename = "short-id")]
    pub sid: String,
}

/// Group including Simple Proxy
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GroupProxy {
    #[serde(rename = "type")]
    pub group_type: String,
    pub name: String,
    pub proxies: Vec<String>,
    #[serde(rename = "include-all", skip_serializing_if = "Option::is_none")]
    pub include_all: Option<bool>,
    #[serde(rename = "exclude-type", skip_serializing_if = "Option::is_none")]
    pub exclude_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<u16>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Mux {
    pub enable: bool,
    pub protocol: String,
    #[serde(rename = "max-connections")]
    pub max_connections: u32,
    #[serde(rename = "min-streams")]
    pub min_streams: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Smux {
    pub enable: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub udp: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mux: Option<Mux>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub smux: Option<Smux>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Provider {
    #[serde(rename = "type")]
    pub provider_type: String,
    pub url: String,
    pub interval: i32,
    proxy: String,
    #[serde(rename = "health-check")]
    pub health: HealthSettings,
    #[serde(rename = "override", skip_serializing_if = "Option::is_none")]
    pub override_settings: Option<ProviderOverride>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthSettings {
    pub enable: bool,
    pub url: String,
    pub interval: i32,
    pub timeout: i32,
    pub lazy: bool,
    #[serde(rename = "expected-status")]
    pub status: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Profile {
    #[serde(rename = "store-selected")]
    pub selected: bool,
    #[serde(rename = "store-fake-ip")]
    pub fake_ip: bool,
}

impl Default for Mihomo {
    fn default() -> Self {
        Self {
            net: Net::default(),
            geo_mode: true,
            geo_update: true,
            geo_update_interval: 24,
            geo_x: Geo::default(),
            level: CONF.app.level.into(),
            find_process: "strict".to_string(),
            external_controller: CONF.mihomo.url.clone(),
            secret: CONF.mihomo.token.expose_secret().to_string(),
            profile: Profile::default(),
            dns: Dns::default(),
            tun: Tun::default(),
            proxies: Vec::new(),
            groups: vec![GroupProxy::default()],
            providers: HashMap::new(),
            rules: Vec::new(),
        }
    }
}

impl Default for Geo {
    fn default() -> Self {
        Self {
            geoip: format!("{DATA_URL}/geoip.dat"),
            geosite: format!("{DATA_URL}/geosite.dat"),
            mmdb: format!("{DATA_URL}/country.mmdb"),
            asn: format!("{DATA_URL}/GeoLite2-ASN.mmdb"),
        }
    }
}

impl Default for Net {
    fn default() -> Self {
        Self {
            allow_lan: false,
            ipv6: true,
            mixed: CONF.mihomo.mixed_port,
            bind_addr: "*".to_string(),
            mode: "rule".to_string(),
            tcp_concurrent: true,
        }
    }
}

impl Default for Dns {
    fn default() -> Self {
        Self {
            enable: true,
            ipv6: false,
            enhanced_mode: "fake-ip".to_string(),
            listen: "0.0.0.0:1053".to_string(),
            fake_range: "198.18.0.1/16".to_string(),
            fake_range6: "fdfe:dcba:9876::1/64".to_string(),
            fake_filter_mode: "rule".to_string(),
            fake_filter: vec![
                "DOMAIN-SUFFIX,lan,real-ip".to_string(),
                "DOMAIN-SUFFIX,local,real-ip".to_string(),
                "DOMAIN-SUFFIX,home,real-ip".to_string(),
                "DOMAIN-SUFFIX,dev,real-ip".to_string(),
                "DOMAIN-SUFFIX,spaces,real-ip".to_string(),
                "DOMAIN-SUFFIX,msftconnecttest.com,fake-ip".to_string(),
                "DOMAIN-SUFFIX,msftncsi.com,fake-ip".to_string(),
                "DOMAIN-SUFFIX,ntp.org,real-ip".to_string(),
                "DOMAIN-SUFFIX,docker.com,fake-ip".to_string(),
                "DOMAIN-SUFFIX,gcr.io,fake-ip".to_string(),
                "MATCH,fake-ip".to_string(),
            ],
            default_ns: vec![
                "8.8.8.8".to_string(),
                "1.1.1.1".to_string(),
                "9.9.9.9".to_string(),
                "77.88.8.8".to_string(),
            ],
            nameserver: vec![
                "https://dns.google/dns-query".to_string(),
                "https://cloudflare-dns.com/dns-query".to_string(),
                "https://dns.yandex.ru/dns-query".to_string(),
                "udp://9.9.9.9:53".to_string(),
            ],
            proxy_ns: vec![
                "https://dns.google/dns-query".to_string(),
                "https://cloudflare-dns.com/dns-query".to_string(),
            ],
            fallback: vec![
                "tls://8.8.4.4".to_string(),
                "tls://1.1.1.1".to_string(),
                "tls://77.88.8.8:853".to_string(),
            ],
        }
    }
}

impl Default for Tun {
    fn default() -> Self {
        Self {
            enable: false,
            device: "homeTun".to_string(),
            stack: "gVisor".to_string(),
            route: true,
            detect: true,
            strict: true,
            dns: vec!["any:53".to_string(), "tcp://any:53".to_string()],
        }
    }
}

impl Default for GroupProxy {
    fn default() -> Self {
        Self {
            group_type: "url-test".to_string(),
            name: "AUTO".to_string(),
            include_all: Some(true),
            exclude_type: Some(DIRECT.to_string()),
            tolerance: Some(150),
            proxies: vec![],
        }
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            selected: true,
            fake_ip: true,
        }
    }
}

impl Default for Provider {
    fn default() -> Self {
        Self {
            provider_type: "http".to_string(),
            url: String::new(),
            interval: 3600,
            proxy: DIRECT.to_string(),
            health: HealthSettings {
                enable: true,
                url: CHECK_URL.to_string(),
                interval: 300,
                timeout: 5000,
                lazy: true,
                status: 204,
            },
            override_settings: Some(ProviderOverride {
                udp: Some(true),
                mux: Some(Mux {
                    enable: true,
                    protocol: "h2mux".to_string(),
                    max_connections: 4,
                    min_streams: 4,
                }),
                smux: Some(Smux { enable: true }),
            }),
        }
    }
}

impl Provider {
    pub(crate) fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            ..Default::default()
        }
    }
}
