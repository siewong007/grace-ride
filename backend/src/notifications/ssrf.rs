//! SSRF-safe DNS-pinned HTTP target resolution and HMAC-SHA256 webhook signing.

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{OnceLock, RwLock},
    time::{Duration, Instant},
};

use ring::hmac;
use url::Url;

static PINNED_CLIENTS: OnceLock<RwLock<HashMap<(String, bool), (Instant, reqwest::Client)>>> =
    OnceLock::new();

pub fn sign_webhook_headers(secret: &str, body: &[u8]) -> (String, Option<String>) {
    let ts = chrono::Utc::now().timestamp().to_string();
    let trimmed = secret.trim();
    if trimmed.is_empty() {
        return (ts, None);
    }
    let key = hmac::Key::new(hmac::HMAC_SHA256, trimmed.as_bytes());
    let mut ctx = hmac::Context::with_key(&key);
    ctx.update(ts.as_bytes());
    ctx.update(b".");
    ctx.update(body);
    let sig = ctx.sign();
    (ts, Some(format!("sha256={}", hex::encode(sig.as_ref()))))
}

fn is_metadata_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4 == std::net::Ipv4Addr::new(169, 254, 169, 254),
        IpAddr::V6(v6) => v6.to_ipv4_mapped() == Some(std::net::Ipv4Addr::new(169, 254, 169, 254)),
    }
}

fn is_non_global_v4(v4: std::net::Ipv4Addr) -> bool {
    let o = v4.octets();
    let cgnat = o[0] == 100 && (o[1] & 0b1100_0000) == 64;
    let bench = o[0] == 198 && (o[1] == 18 || o[1] == 19);
    let this_net = o[0] == 0;
    this_net
        || v4.is_private()
        || v4.is_loopback()
        || v4.is_link_local()
        || v4.is_broadcast()
        || v4.is_documentation()
        || v4.is_multicast()
        || v4.is_unspecified()
        || cgnat
        || bench
        || o[0] >= 240
}

fn is_non_global_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_non_global_v4(v4),
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_non_global_v4(mapped);
            }
            let segs = v6.segments();
            let is_ipv4_compat =
                segs[0..6] == [0, 0, 0, 0, 0, 0] && !v6.is_loopback() && !v6.is_unspecified();
            if is_ipv4_compat {
                let mapped = std::net::Ipv4Addr::new(
                    (segs[6] >> 8) as u8,
                    segs[6] as u8,
                    (segs[7] >> 8) as u8,
                    segs[7] as u8,
                );
                return is_non_global_v4(mapped);
            }
            let unicast_link_local = (segs[0] & 0xffc0) == 0xfe80;
            let site_local = (segs[0] & 0xffc0) == 0xfec0;
            let unique_local = (segs[0] & 0xfe00) == 0xfc00;
            let doc = segs[0] == 0x2001 && segs[1] == 0x0db8;
            let nat64_well_known = segs[0] == 0x0064
                && segs[1] == 0xff9b
                && segs[2] == 0
                && segs[3] == 0
                && segs[4] == 0
                && segs[5] == 0;
            let nat64_local = segs[0] == 0x0064 && segs[1] == 0xff9b && segs[2] == 0x0001;
            let six_to_four = segs[0] == 0x2002;
            let ipv4_translated = segs[0..4] == [0, 0, 0, 0] && segs[4] == 0xffff && segs[5] == 0;
            let discard = segs[0] == 0x0100 && segs[1] == 0 && segs[2] == 0 && segs[3] == 0;
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || unicast_link_local
                || site_local
                || unique_local
                || doc
                || nat64_well_known
                || nat64_local
                || six_to_four
                || ipv4_translated
                || discard
        }
    }
}

pub async fn resolve_and_validate_http_target(
    raw: &str,
    max_len: usize,
    dev_mode: bool,
) -> Option<(Url, Option<(String, Vec<SocketAddr>)>)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > max_len {
        return None;
    }
    let parsed = Url::parse(trimmed).ok()?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return None;
    }
    let scheme = parsed.scheme();
    if dev_mode {
        if scheme != "https" && scheme != "http" {
            return None;
        }
    } else if scheme != "https" {
        return None;
    }
    let host = parsed.host()?;
    let host_lower = parsed
        .host_str()
        .unwrap_or_default()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host_lower.is_empty()
        || host_lower == "metadata.google.internal"
        || host_lower == "169.254.169.254"
    {
        return None;
    }
    match host {
        url::Host::Ipv4(v4) => {
            let ip = IpAddr::V4(v4);
            if is_metadata_ip(ip) || (!dev_mode && is_non_global_v4(v4)) {
                return None;
            }
            Some((parsed, None))
        }
        url::Host::Ipv6(v6) => {
            let ip = IpAddr::V6(v6);
            if is_metadata_ip(ip) || (!dev_mode && is_non_global_ip(ip)) {
                return None;
            }
            Some((parsed, None))
        }
        url::Host::Domain(domain) => {
            if domain.ends_with('.') {
                return None;
            }
            let d = domain.to_ascii_lowercase();
            if d == "metadata.google.internal" {
                return None;
            }
            if dev_mode {
                return Some((parsed, None));
            }
            if d == "localhost"
                || d.ends_with(".localhost")
                || d.ends_with(".local")
                || d.ends_with(".internal")
                || !d.contains('.')
            {
                return None;
            }
            let port = parsed.port_or_known_default().unwrap_or(443);
            let addrs = tokio::net::lookup_host((d.as_str(), port)).await.ok()?;
            let mut valid = Vec::new();
            for addr in addrs {
                if is_non_global_ip(addr.ip()) {
                    return None;
                }
                valid.push(addr);
            }
            if valid.is_empty() {
                return None;
            }
            Some((parsed, Some((d, valid))))
        }
    }
}

pub async fn build_pinned_client_with_url(
    raw_url: &str,
    dev_mode: bool,
) -> Option<(String, reqwest::Client)> {
    let (parsed, pin) = resolve_and_validate_http_target(raw_url, 1000, dev_mode).await?;
    let target_url = parsed.to_string();
    let key = (target_url.clone(), dev_mode);
    let cache = PINNED_CLIENTS.get_or_init(|| RwLock::new(HashMap::new()));
    if let Ok(guard) = cache.read() {
        if let Some((at, client)) = guard.get(&key) {
            if at.elapsed() < Duration::from_secs(60) {
                return Some((target_url, client.clone()));
            }
        }
    }

    let client = match pin {
        Some((domain, addrs)) => reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(5))
            .resolve_to_addrs(&domain, &addrs)
            .build()
            .ok()?,
        None => crate::util::alert_client().clone(),
    };
    if let Ok(mut guard) = cache.write() {
        if guard.len() >= 32 {
            guard.clear();
        }
        guard.insert(key, (Instant::now(), client.clone()));
    }
    Some((target_url, client))
}

pub async fn build_pinned_client(raw_url: &str, dev_mode: bool) -> Option<reqwest::Client> {
    build_pinned_client_with_url(raw_url, dev_mode)
        .await
        .map(|(_, c)| c)
}
