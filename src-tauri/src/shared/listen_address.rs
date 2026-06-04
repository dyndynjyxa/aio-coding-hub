//! Usage: Shared parsing helpers for gateway listen addresses.

#[derive(Debug, Clone)]
pub(crate) struct ParsedListenAddress {
    pub(crate) host: String,
    pub(crate) port: Option<u16>,
}

pub(crate) fn is_wildcard_host(host: &str) -> bool {
    matches!(host.trim(), "0.0.0.0" | "::")
}

pub(crate) fn format_host_port(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

pub(crate) fn parse_custom_listen_address(
    input: &str,
) -> crate::shared::error::AppResult<ParsedListenAddress> {
    let raw = input.trim();
    if raw.is_empty() {
        return Ok(ParsedListenAddress {
            host: "0.0.0.0".to_string(),
            port: None,
        });
    }
    if raw.contains("://") || raw.contains('/') {
        return Err(
            "SEC_INVALID_INPUT: custom listen address must be host or host:port"
                .to_string()
                .into(),
        );
    }

    if let Some(rest) = raw.strip_prefix('[') {
        let idx = rest.find(']').ok_or_else(|| {
            "SEC_INVALID_INPUT: invalid IPv6 address: missing closing ']'".to_string()
        })?;
        let host = rest[..idx].trim();
        if host.is_empty() {
            return Err("SEC_INVALID_INPUT: custom listen address missing host"
                .to_string()
                .into());
        }
        let tail = rest[idx + 1..].trim();
        if tail.is_empty() {
            return Ok(ParsedListenAddress {
                host: host.to_string(),
                port: None,
            });
        }
        let port_raw = tail
            .strip_prefix(':')
            .ok_or_else(|| {
                "SEC_INVALID_INPUT: custom listen address must be [ipv6]:port".to_string()
            })?
            .trim();
        let port: u16 = port_raw
            .parse()
            .map_err(|_| "SEC_INVALID_INPUT: invalid custom listen port".to_string())?;
        if port < 1024 {
            return Err("SEC_INVALID_INPUT: custom listen port must be >= 1024"
                .to_string()
                .into());
        }
        return Ok(ParsedListenAddress {
            host: host.to_string(),
            port: Some(port),
        });
    }

    let parts: Vec<&str> = raw.split(':').collect();
    if parts.len() == 1 {
        return Ok(ParsedListenAddress {
            host: raw.to_string(),
            port: None,
        });
    }
    if parts.len() == 2 {
        let host = parts[0].trim();
        if host.is_empty() {
            return Err("SEC_INVALID_INPUT: custom listen address missing host"
                .to_string()
                .into());
        }
        let port_raw = parts[1].trim();
        let port: u16 = port_raw
            .parse()
            .map_err(|_| "SEC_INVALID_INPUT: invalid custom listen port".to_string())?;
        if port < 1024 {
            return Err("SEC_INVALID_INPUT: custom listen port must be >= 1024"
                .to_string()
                .into());
        }
        return Ok(ParsedListenAddress {
            host: host.to_string(),
            port: Some(port),
        });
    }

    Err("SEC_INVALID_INPUT: IPv6 must use [addr]:port"
        .to_string()
        .into())
}

pub(crate) fn parse_custom_host_address(
    input: &str,
) -> crate::shared::error::AppResult<Option<String>> {
    let raw = input.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    if raw.contains("://") || raw.contains('/') {
        return Err(
            "SEC_INVALID_INPUT: custom host address must be host without scheme, path, or port"
                .to_string()
                .into(),
        );
    }

    if let Some(rest) = raw.strip_prefix('[') {
        let idx = rest.find(']').ok_or_else(|| {
            "SEC_INVALID_INPUT: invalid IPv6 host address: missing closing ']'".to_string()
        })?;
        let host = rest[..idx].trim();
        if host.is_empty() {
            return Err("SEC_INVALID_INPUT: custom host address missing host"
                .to_string()
                .into());
        }
        let tail = rest[idx + 1..].trim();
        if !tail.is_empty() {
            return Err(
                "SEC_INVALID_INPUT: custom host address must not include a port"
                    .to_string()
                    .into(),
            );
        }
        return Ok(Some(host.to_string()));
    }

    if raw.contains('[') || raw.contains(']') {
        return Err("SEC_INVALID_INPUT: invalid custom host address brackets"
            .to_string()
            .into());
    }

    if raw.contains(':') {
        raw.parse::<std::net::Ipv6Addr>().map_err(|_| {
            "SEC_INVALID_INPUT: custom host address must be a hostname, IPv4, or IPv6 host without port"
                .to_string()
        })?;
    }

    Ok(Some(raw.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- is_wildcard_host --

    #[test]
    fn wildcard_host_detects_ipv4_any() {
        assert!(is_wildcard_host("0.0.0.0"));
    }

    #[test]
    fn wildcard_host_detects_ipv6_any() {
        assert!(is_wildcard_host("::"));
    }

    #[test]
    fn wildcard_host_rejects_localhost() {
        assert!(!is_wildcard_host("127.0.0.1"));
    }

    #[test]
    fn wildcard_host_trims_whitespace() {
        assert!(is_wildcard_host("  0.0.0.0  "));
    }

    // -- format_host_port --

    #[test]
    fn format_host_port_ipv4() {
        assert_eq!(format_host_port("127.0.0.1", 8080), "127.0.0.1:8080");
    }

    #[test]
    fn format_host_port_ipv6_wraps_in_brackets() {
        assert_eq!(format_host_port("::1", 8080), "[::1]:8080");
    }

    #[test]
    fn format_host_port_hostname() {
        assert_eq!(format_host_port("localhost", 3000), "localhost:3000");
    }

    // -- parse_custom_listen_address --

    #[test]
    fn parse_empty_defaults_to_wildcard() {
        let result = parse_custom_listen_address("").unwrap();
        assert_eq!(result.host, "0.0.0.0");
        assert!(result.port.is_none());
    }

    #[test]
    fn parse_whitespace_only_defaults_to_wildcard() {
        let result = parse_custom_listen_address("   ").unwrap();
        assert_eq!(result.host, "0.0.0.0");
        assert!(result.port.is_none());
    }

    #[test]
    fn parse_host_only() {
        let result = parse_custom_listen_address("192.168.1.1").unwrap();
        assert_eq!(result.host, "192.168.1.1");
        assert!(result.port.is_none());
    }

    #[test]
    fn parse_host_and_port() {
        let result = parse_custom_listen_address("192.168.1.1:8080").unwrap();
        assert_eq!(result.host, "192.168.1.1");
        assert_eq!(result.port, Some(8080));
    }

    #[test]
    fn parse_rejects_port_below_1024() {
        assert!(parse_custom_listen_address("127.0.0.1:80").is_err());
    }

    #[test]
    fn parse_rejects_url_with_scheme() {
        assert!(parse_custom_listen_address("http://127.0.0.1:8080").is_err());
    }

    #[test]
    fn parse_rejects_url_with_path() {
        assert!(parse_custom_listen_address("127.0.0.1/path").is_err());
    }

    #[test]
    fn parse_ipv6_bracket_notation_host_only() {
        let result = parse_custom_listen_address("[::1]").unwrap();
        assert_eq!(result.host, "::1");
        assert!(result.port.is_none());
    }

    #[test]
    fn parse_ipv6_bracket_notation_with_port() {
        let result = parse_custom_listen_address("[::1]:8080").unwrap();
        assert_eq!(result.host, "::1");
        assert_eq!(result.port, Some(8080));
    }

    #[test]
    fn parse_ipv6_bracket_rejects_low_port() {
        assert!(parse_custom_listen_address("[::1]:80").is_err());
    }

    #[test]
    fn parse_ipv6_bracket_rejects_missing_closing_bracket() {
        assert!(parse_custom_listen_address("[::1").is_err());
    }

    #[test]
    fn parse_ipv6_bracket_rejects_empty_host() {
        assert!(parse_custom_listen_address("[]").is_err());
    }

    #[test]
    fn parse_bare_ipv6_rejects_ambiguous_colons() {
        // Bare IPv6 without brackets should be rejected.
        assert!(parse_custom_listen_address("::1:8080:extra").is_err());
    }

    // -- parse_custom_host_address --

    #[test]
    fn parse_custom_host_empty_returns_none() {
        assert_eq!(parse_custom_host_address("   ").unwrap(), None);
    }

    #[test]
    fn parse_custom_host_accepts_hostname() {
        assert_eq!(
            parse_custom_host_address("devbox.internal").unwrap(),
            Some("devbox.internal".to_string())
        );
    }

    #[test]
    fn parse_custom_host_accepts_ipv4() {
        assert_eq!(
            parse_custom_host_address("172.20.80.1").unwrap(),
            Some("172.20.80.1".to_string())
        );
    }

    #[test]
    fn parse_custom_host_accepts_bare_ipv6() {
        assert_eq!(
            parse_custom_host_address("::1").unwrap(),
            Some("::1".to_string())
        );
    }

    #[test]
    fn parse_custom_host_accepts_bracketed_ipv6() {
        assert_eq!(
            parse_custom_host_address("[::1]").unwrap(),
            Some("::1".to_string())
        );
    }

    #[test]
    fn parse_custom_host_rejects_url() {
        assert!(parse_custom_host_address("http://127.0.0.1").is_err());
    }

    #[test]
    fn parse_custom_host_rejects_path() {
        assert!(parse_custom_host_address("127.0.0.1/path").is_err());
    }

    #[test]
    fn parse_custom_host_rejects_ipv4_port() {
        assert!(parse_custom_host_address("127.0.0.1:37123").is_err());
    }

    #[test]
    fn parse_custom_host_rejects_bracketed_ipv6_port() {
        assert!(parse_custom_host_address("[::1]:37123").is_err());
    }

    #[test]
    fn parse_custom_host_rejects_invalid_bare_ipv6() {
        assert!(parse_custom_host_address("::1:8080:extra").is_err());
    }
}
