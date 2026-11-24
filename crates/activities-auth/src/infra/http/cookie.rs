use axum::http::HeaderMap;
use cookie::{Cookie, SameSite, time::OffsetDateTime};

#[derive(Debug, Clone)]
pub struct CookieConfig {
    /// Whether cookies may be marked `Secure` at all (see [`CookieConfig::for_request`]).
    pub secure: bool,
    pub same_site: SameSite,
    pub http_only: bool,
    pub domain: Option<String>,
}

impl Default for CookieConfig {
    fn default() -> Self {
        Self {
            secure: true,
            same_site: SameSite::Strict,
            http_only: true,
            domain: None,
        }
    }
}

impl CookieConfig {
    /// Returns this configuration adjusted to the scheme the request was received on.
    ///
    /// A cookie is only marked `Secure` when the request reached us over HTTPS, either directly or
    /// through a TLS-terminating reverse proxy that forwarded the original scheme in
    /// `X-Forwarded-Proto`. Without this, plain-HTTP deployments (the single-user modes, the
    /// multi-user modes run over a private tunnel, and local development) would have their session
    /// cookie silently dropped by the browser, so every subsequent request would answer `401`.
    pub fn for_request(&self, headers: &HeaderMap) -> Self {
        Self {
            secure: self.secure && request_is_https(headers),
            ..self.clone()
        }
    }
}

/// Whether the request reached us over HTTPS, as reported by the reverse proxy in
/// `X-Forwarded-Proto` (a comma-separated list, of which the first value is the client-facing
/// scheme). When the header is absent we assume plain HTTP, which keeps cookie handling working in
/// direct/local deployments while still hardening it behind a TLS-terminating proxy.
fn request_is_https(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .is_some_and(|proto| proto.trim().eq_ignore_ascii_case("https"))
}

/// Builds a cookie that instructs the browser to immediately drop `name`, for use by logout
/// handlers. Attributes (path/domain/etc) must match the cookie that was originally set for the
/// browser to actually overwrite it.
pub fn build_removal_cookie(name: &'static str, cookie_config: &CookieConfig) -> Cookie<'static> {
    let mut builder = Cookie::build((name, ""))
        .expires(OffsetDateTime::UNIX_EPOCH)
        .secure(cookie_config.secure)
        .http_only(cookie_config.http_only)
        .same_site(cookie_config.same_site)
        .path("/");
    if let Some(domain) = cookie_config.domain.clone() {
        builder = builder.domain(domain);
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};

    use super::*;

    #[test]
    fn cookie_config_is_secure_only_when_request_is_https() {
        let config = CookieConfig::default();

        assert!(!config.for_request(&HeaderMap::new()).secure);

        let mut headers = HeaderMap::new();
        for proto in ["https", "HTTPS", " https ", "https, http"] {
            headers.insert("x-forwarded-proto", HeaderValue::from_static(proto));
            assert!(
                config.for_request(&headers).secure,
                "`{proto}` should be treated as HTTPS"
            );
        }

        for proto in ["http", "http, https", ""] {
            headers.insert("x-forwarded-proto", HeaderValue::from_static(proto));
            assert!(
                !config.for_request(&headers).secure,
                "`{proto}` should not be treated as HTTPS"
            );
        }
    }

    #[test]
    fn cookie_config_never_marks_cookies_secure_when_disabled() {
        let config = CookieConfig {
            secure: false,
            ..CookieConfig::default()
        };
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));

        assert!(!config.for_request(&headers).secure);
    }
}
