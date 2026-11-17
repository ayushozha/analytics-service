//! SSRF protection for project-controlled outbound URLs (reverse-ETL destinations,
//! alert notification channels, and webhooks). Rejects non-http(s) schemes, localhost /
//! cloud-metadata hostnames, and literal IPs in private, loopback, link-local, or other
//! reserved ranges (including the 169.254.169.254 cloud-metadata endpoint).
//!
//! This is a synchronous, literal-host check applied at configuration time. Defending
//! against DNS rebinding (resolving + pinning at request time) is a separate, future hardening.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use url::Host;

/// Domain suffixes that must never be outbound targets.
const BLOCKED_HOST_SUFFIXES: &[&str] = &[".local", ".internal", ".lan", ".localhost"];

/// Validate that `raw` is a public http(s) URL safe to send project-controlled requests to.
/// Returns a human-readable reason on rejection.
pub fn ensure_public_http_url(raw: &str) -> Result<(), String> {
    let parsed = url::Url::parse(raw).map_err(|_| "URL is not valid".to_string())?;

    match parsed.scheme() {
        "http" | "https" => {}
        other => {
            return Err(format!(
                "URL scheme '{other}' is not allowed; use http or https"
            ))
        }
    }

    match parsed.host() {
        Some(Host::Ipv4(ip)) => {
            if is_blocked_ip(IpAddr::V4(ip)) {
                return Err(
                    "URL cannot target a private, local, or reserved IP address".to_string()
                );
            }
        }
        Some(Host::Ipv6(ip)) => {
            if is_blocked_ip(IpAddr::V6(ip)) {
                return Err(
                    "URL cannot target a private, local, or reserved IP address".to_string()
                );
            }
        }
        Some(Host::Domain(domain)) => {
            let host = domain.to_ascii_lowercase();
            if host == "localhost"
                || BLOCKED_HOST_SUFFIXES
                    .iter()
                    .any(|suffix| host.ends_with(suffix))
            {
                return Err(format!("URL host '{host}' is not an allowed target"));
            }
        }
        None => return Err("URL must include a host".to_string()),
    }

    Ok(())
}

/// Whether an IP is in a private, loopback, link-local, or otherwise reserved range that
/// should never be reachable from a project-controlled outbound request.
pub fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_blocked_ipv4(ip),
        IpAddr::V6(ip) => is_blocked_ipv6(ip),
    }
}

fn is_blocked_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_unspecified()
        || ip.is_multicast()
        || octets[0] == 0
        // 100.64.0.0/10 carrier-grade NAT
        || (octets[0] == 100 && (octets[1] & 0b1100_0000) == 64)
        // 198.18.0.0/15 benchmarking
        || (octets[0] == 198 && matches!(octets[1], 18 | 19))
        // 192.0.0.0/24 IETF protocol assignments
        || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
}

fn is_blocked_ipv6(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        // fc00::/7 unique local
        || (segments[0] & 0xfe00) == 0xfc00
        // fe80::/10 link-local
        || (segments[0] & 0xffc0) == 0xfe80
}

#[cfg(test)]
mod tests {
    use super::ensure_public_http_url;

    #[test]
    fn allows_public_https_endpoints() {
        assert!(ensure_public_http_url("https://example.com/events").is_ok());
        assert!(ensure_public_http_url("http://hooks.slack.com/services/abc").is_ok());
        assert!(ensure_public_http_url("https://8.8.8.8/ingest").is_ok());
    }

    #[test]
    fn blocks_cloud_metadata_and_localhost() {
        assert!(ensure_public_http_url("http://169.254.169.254/latest/meta-data/").is_err());
        assert!(ensure_public_http_url("http://localhost:9000/x").is_err());
        assert!(ensure_public_http_url("http://127.0.0.1/x").is_err());
        assert!(ensure_public_http_url("http://[::1]/x").is_err());
    }

    #[test]
    fn blocks_private_and_reserved_ranges() {
        assert!(ensure_public_http_url("http://10.0.0.5/x").is_err());
        assert!(ensure_public_http_url("http://192.168.1.1/x").is_err());
        assert!(ensure_public_http_url("http://172.16.0.1/x").is_err());
        assert!(ensure_public_http_url("http://100.64.0.1/x").is_err());
        assert!(ensure_public_http_url("http://0.0.0.0/x").is_err());
    }

    #[test]
    fn blocks_internal_hostnames_and_bad_schemes() {
        assert!(ensure_public_http_url("https://projects-db.internal/x").is_err());
        assert!(ensure_public_http_url("https://api.cluster.local/x").is_err());
        assert!(ensure_public_http_url("ftp://example.com/x").is_err());
        assert!(ensure_public_http_url("not a url").is_err());
    }
}
