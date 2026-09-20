//! Static virtual-host matching. No allocation in the ordinary match path.

fn hostname(authority: &str) -> Option<&str> {
    if authority.is_empty()
        || authority
            .bytes()
            .any(|b| b.is_ascii_whitespace() || b == b'@' || b == b'/' || b == b'\\')
    {
        return None;
    }
    if authority.starts_with('[') {
        let close = authority.find(']')?;
        let address = &authority[1..close];
        address.parse::<std::net::Ipv6Addr>().ok()?;
        let rest = &authority[close + 1..];
        if !rest.is_empty() {
            rest.strip_prefix(':')?.parse::<u16>().ok()?;
        }
        return Some(address);
    }
    let host = if let Some((host, port)) = authority.split_once(':') {
        port.parse::<u16>().ok()?;
        host
    } else {
        authority
    };
    let host = host.strip_suffix('.').unwrap_or(host);
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}

pub fn valid_pattern(pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern.starts_with('[') {
        return pattern.ends_with(']') && hostname(pattern).is_some();
    }
    let domain = pattern.strip_prefix("*.").unwrap_or(pattern);
    let domain = domain.strip_suffix('.').unwrap_or(domain);
    !domain.is_empty()
        && domain.len() <= 253
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

pub fn matches(pattern: &str, authority: &str) -> bool {
    let Some(host) = hostname(authority) else {
        return false;
    };
    if pattern == "*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        let suffix = suffix.strip_suffix('.').unwrap_or(suffix);
        return host.len() > suffix.len() + 1
            && host.as_bytes()[host.len() - suffix.len() - 1] == b'.'
            && host[host.len() - suffix.len()..].eq_ignore_ascii_case(suffix);
    }
    if pattern.starts_with('[') {
        return hostname(pattern)
            .and_then(|host| host.parse::<std::net::Ipv6Addr>().ok())
            .is_some_and(|expected| host.parse::<std::net::Ipv6Addr>().ok() == Some(expected));
    }
    host.eq_ignore_ascii_case(pattern.strip_suffix('.').unwrap_or(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn static_host_boundaries_and_authorities() {
        for (pattern, host) in [
            ("example.com", "EXAMPLE.COM.:443"),
            ("*.example.com", "a.b.example.com:80"),
            ("[::1]", "[0:0:0:0:0:0:0:1]:8080"),
        ] {
            assert!(valid_pattern(pattern));
            assert!(matches(pattern, host));
        }
        for host in [
            "example.com",
            "evil-example.com",
            "a.example.com.evil",
            "",
            "a.example.com:bad",
            "a.example.com@evil",
            "a.example.com/",
            "a.example.com:443:80",
        ] {
            assert!(!matches("*.example.com", host), "{host}");
        }
        for pattern in [
            "",
            "https://example.com",
            "example.com:80",
            "*example.com",
            "example..com",
            "-bad.example",
            "[::1]:80",
        ] {
            assert!(!valid_pattern(pattern), "{pattern}");
        }
    }
}
