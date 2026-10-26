//! The cookie that remembers the session.
//!
//! ## Why a cookie and not something in the client
//!
//! The storefront is a web app on the same origin as the API, and a browser
//! sends its cookies on every request to that origin **by itself**. So the
//! server can remember the session without the client storing anything: the
//! client asks "am I still the shopper I was?", the server reads the cookie
//! it set earlier, and the client is told.
//!
//! The alternative — the client keeping the session id in its own storage —
//! needs a platform plugin on every client (a browser's local storage, the
//! app's preferences), and every one of them is a place the session id can
//! leak from. A cookie that only the server can read does not have that
//! problem.
//!
//! ## The cookie is `HttpOnly`
//!
//! The session id is also the cart's id and the buyer's key, so a script on
//! the page must not be able to read it. `HttpOnly` says exactly that: the
//! browser keeps it and sends it, and no script sees it.
//!
//! The client never needs to read it either — it learns the session id from
//! the response body, exactly as it always has.
//!
//! ## What it is not
//!
//! It is not an authentication credential on its own: the session id is still
//! required in the `x-session-id` header for the services that scope by it
//! (the cart and the order). The cookie only lets the client **find** its
//! session again after a reload.

use axum::http::HeaderMap;

/// The cookie the session id is kept in.
pub const SESSION_COOKIE: &str = "dokipon_session";

/// The `Set-Cookie` value that remembers `session_id` for `max_age_seconds`.
///
/// The lifetime is the session's own remaining lifetime: a cookie that
/// outlived its session would keep sending an id the server refuses, and a
/// cookie that died before the session would lose the session early.
pub fn set_cookie(session_id: &str, max_age_seconds: i64) -> String {
    let max_age = max_age_seconds.max(0);
    format!(
        "{SESSION_COOKIE}={session_id}; Path=/; Max-Age={max_age}; HttpOnly; Secure; SameSite=Lax"
    )
}

/// The session id a request carries.
///
/// The header first — it is the contract's carrier — and the cookie second.
/// A request that presents the header is the client that already knows its
/// session; the cookie is how a client that **forgot** it (a reload) finds
/// it again.
pub fn presented_session(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-session-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| cookie_value(headers, SESSION_COOKIE))
}

/// One cookie's value, by name.
pub fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    for header in headers.get_all("cookie") {
        let Ok(raw) = header.to_str() else {
            continue;
        };
        for pair in raw.split(';') {
            let pair = pair.trim();
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            if key.trim() == name {
                let value = value.trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers_with(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.append(
                axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        headers
    }

    #[test]
    fn the_header_is_the_first_choice() {
        let headers = headers_with(&[
            ("x-session-id", "sess-000001"),
            ("cookie", "dokipon_session=sess-000002"),
        ]);
        assert_eq!(presented_session(&headers).as_deref(), Some("sess-000001"));
    }

    #[test]
    fn the_cookie_is_the_fallback() {
        let headers = headers_with(&[("cookie", "dokipon_session=sess-000002")]);
        assert_eq!(presented_session(&headers).as_deref(), Some("sess-000002"));
    }

    #[test]
    fn a_cookie_among_others_is_found() {
        let headers =
            headers_with(&[("cookie", "other=1; dokipon_session=sess-000003; another=2")]);
        assert_eq!(presented_session(&headers).as_deref(), Some("sess-000003"));
    }

    #[test]
    fn an_empty_header_does_not_count() {
        let headers = headers_with(&[("x-session-id", "   ")]);
        assert_eq!(presented_session(&headers), None);
    }

    #[test]
    fn the_cookie_is_http_only_and_secure() {
        let cookie = set_cookie("sess-000001", 1800);
        assert!(cookie.contains("HttpOnly"), "{cookie}");
        assert!(cookie.contains("Secure"), "{cookie}");
        assert!(cookie.contains("SameSite=Lax"), "{cookie}");
        assert!(cookie.contains("Max-Age=1800"), "{cookie}");
        assert!(cookie.contains("Path=/"), "{cookie}");
    }
}
