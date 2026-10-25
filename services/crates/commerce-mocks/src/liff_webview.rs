//! WebView host matrix — serves `CASE-E2E-002`, `CASE-E2E-003`, `CASE-E2E-004`.
//!
//! The LINE channels are distinguished almost entirely by *which host is
//! running the app*, not by anything in the app's own code. This mock is that
//! host: it serves the Flutter web build under a requested host profile, and
//! answers the capability probe the client performs at boot.
//!
//! | Profile | `isInClient` | `isLoggedIn` | Expected client behaviour |
//! |---|---|---|---|
//! | `line_ios` | true | true | full storefront + checkout |
//! | `line_ios_logged_out` | true | false | browse, prompt at checkout |
//! | `line_android` | true | true | full storefront + checkout |
//! | `line_external` | false | false | browse only, **no** LINE APIs |
//! | `line_revoked` | true | false | same as logged-out, consent was withdrawn |
//! | `plain_chrome` | false | false | plain Flutter Web |
//! | `sdk_offline` | — | — | SDK load fails → degrade, no redirect |
//!
//! Each profile also reports which APIs are *available*, because the design
//! notes that a LINE WebView and an external browser do not expose the same set.
//! A client that calls a client-only API outside LINE must be detected.

use axum::{
    extract::Query,
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostProfile {
    LineIos,
    LineIosLoggedOut,
    LineAndroid,
    LineExternal,
    LineRevoked,
    PlainChrome,
    SdkOffline,
    SdkTimeout,
}

#[derive(Debug, Clone, Serialize)]
pub struct Capabilities {
    /// `liff.isInClient()`
    pub is_in_client: bool,
    /// `liff.isLoggedIn()`
    pub is_logged_in: bool,
    /// May the client call `liff.getIDToken()`?
    pub can_request_id_token: bool,
    /// Is a one-shot id_token available without a further round-trip?
    pub has_id_token: bool,
    /// Does `liff.ready()` hold?
    pub sdk_ready: bool,
    /// WebView-specific quirks the client must tolerate.
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostMatrixResponse {
    pub profile: HostProfile,
    pub user_agent: String,
    pub viewport: serde_json::Value,
    pub safe_area_insets: serde_json::Value,
    pub capabilities: Capabilities,
    /// API availability, mirroring the real asymmetry between hosts.
    pub api_availability: HashMap<String, bool>,
}

pub fn capabilities_for(profile: HostProfile) -> Capabilities {
    match profile {
        HostProfile::LineIos => Capabilities {
            is_in_client: true,
            is_logged_in: true,
            can_request_id_token: true,
            has_id_token: true,
            sdk_ready: true,
            notes: vec![
                "WKWebView: the LINE header owns the top inset".into(),
                "Back / minimise / close are handled by the LINE header first.".into(),
            ],
        },
        HostProfile::LineIosLoggedOut => Capabilities {
            is_in_client: true,
            is_logged_in: false,
            can_request_id_token: false,
            has_id_token: false,
            sdk_ready: true,
            notes: vec!["User is in LINE but has not granted account access.".into()],
        },
        HostProfile::LineAndroid => Capabilities {
            is_in_client: true,
            is_logged_in: true,
            can_request_id_token: true,
            has_id_token: true,
            sdk_ready: true,
            notes: vec![
                "Android WebView: keyboard resize differs from iOS".into(),
                "Viewport must be measured after soft-keyboard open.".into(),
            ],
        },
        HostProfile::LineExternal => Capabilities {
            is_in_client: false,
            is_logged_in: false,
            can_request_id_token: false,
            has_id_token: false,
            sdk_ready: true,
            notes: vec![
                "Opened in an external browser: client-only APIs must not be called.".into(),
                "Guest browsing must remain available.".into(),
            ],
        },
        HostProfile::LineRevoked => Capabilities {
            is_in_client: true,
            is_logged_in: false,
            can_request_id_token: false,
            has_id_token: false,
            sdk_ready: true,
            notes: vec![
                "Consent was revoked.".into(),
                "Any already-authorized data must be discarded, not reused.".into(),
            ],
        },
        HostProfile::PlainChrome => Capabilities {
            is_in_client: false,
            is_logged_in: false,
            can_request_id_token: false,
            has_id_token: false,
            sdk_ready: false,
            notes: vec!["Not a LINE surface at all: plain Flutter Web.".into()],
        },
        HostProfile::SdkOffline => Capabilities {
            is_in_client: false,
            is_logged_in: false,
            can_request_id_token: false,
            has_id_token: false,
            sdk_ready: false,
            notes: vec![
                "LIFF SDK failed to load (offline / CSP / network).".into(),
                "The client must degrade to plain web WITHOUT redirecting or reloading.".into(),
            ],
        },
        HostProfile::SdkTimeout => Capabilities {
            is_in_client: false,
            is_logged_in: false,
            can_request_id_token: false,
            has_id_token: false,
            sdk_ready: false,
            notes: vec![
                "SDK load timed out.".into(),
                "A reload loop here is a defect: degrade once and continue.".into(),
            ],
        },
    }
}

fn ua_for(profile: HostProfile) -> &'static str {
    match profile {
        HostProfile::LineIos | HostProfile::LineIosLoggedOut => {
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148 Safari/604.1"
        }
        HostProfile::LineAndroid | HostProfile::LineRevoked => {
            "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36"
        }
        _ => "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    }
}

fn viewport_for(profile: HostProfile) -> serde_json::Value {
    // The LINE client renders in a WebView with its own header, so the usable
    // height is shorter than the device height.
    match profile {
        HostProfile::LineIos | HostProfile::LineIosLoggedOut => {
            serde_json::json!({"width": 375, "height": 667, "devicePixelRatio": 3.0})
        }
        HostProfile::LineAndroid | HostProfile::LineRevoked => {
            serde_json::json!({"width": 412, "height": 892, "devicePixelRatio": 2.625})
        }
        _ => serde_json::json!({"width": 1280, "height": 800, "devicePixelRatio": 1.0}),
    }
}

fn insets_for(profile: HostProfile) -> serde_json::Value {
    match profile {
        HostProfile::LineIos | HostProfile::LineIosLoggedOut => {
            serde_json::json!({"top": 44, "bottom": 34, "left": 0, "right": 0})
        }
        HostProfile::LineAndroid | HostProfile::LineRevoked => {
            serde_json::json!({"top": 24, "bottom": 24, "left": 0, "right": 0})
        }
        _ => serde_json::json!({"top": 0, "bottom": 0, "left": 0, "right": 0}),
    }
}

fn api_availability(profile: HostProfile) -> HashMap<String, bool> {
    let in_client = matches!(
        profile,
        HostProfile::LineIos
            | HostProfile::LineIosLoggedOut
            | HostProfile::LineAndroid
            | HostProfile::LineRevoked
    );
    let logged_in = matches!(profile, HostProfile::LineIos | HostProfile::LineAndroid);
    HashMap::from([
        ("liff.isInClient".into(), true),
        ("liff.isLoggedIn".into(), logged_in),
        ("liff.getIDToken".into(), logged_in),
        ("liff.getProfile".into(), logged_in),
        ("liff.sendMessages".into(), in_client),
        ("liff.closeWindow".into(), in_client),
        ("liff.clearCache".into(), in_client),
        // Never available: these are LINE *server* APIs requiring a secret the
        // client must never hold. Their absence is why the Channel Secret
        // cannot leak from a front-end bundle by accident.
        ("line.channelSecret".into(), false),
        ("line.verifySignature".into(), false),
    ])
}

#[derive(Debug, Deserialize)]
struct ProfileQuery {
    #[serde(default)]
    profile: Option<HostProfile>,
}

pub fn router() -> Router {
    Router::new()
        .route(
            "/__mock/host",
            get(
                move |Query(q): Query<ProfileQuery>, headers: HeaderMap| async move {
                    let profile = q.profile.unwrap_or(HostProfile::PlainChrome);
                    let body = HostMatrixResponse {
                        profile,
                        user_agent: headers
                            .get("user-agent")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or(ua_for(profile))
                            .to_string(),
                        viewport: viewport_for(profile),
                        safe_area_insets: insets_for(profile),
                        capabilities: capabilities_for(profile),
                        api_availability: api_availability(profile),
                    };
                    (StatusCode::OK, Json(body))
                },
            ),
        )
        .route(
            "/__mock/profiles",
            get(|| async {
                Json(serde_json::json!({
                    "profiles": [
                        "line_ios", "line_ios_logged_out", "line_android",
                        "line_external", "line_revoked", "plain_chrome",
                        "sdk_offline", "sdk_timeout"
                    ],
                    "usage": "GET /__mock/host?profile=line_external"
                }))
            }),
        )
        .route("/healthz", get(|| async { StatusCode::OK }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_ios_supports_the_full_flow() {
        let c = capabilities_for(HostProfile::LineIos);
        assert!(c.is_in_client && c.is_logged_in && c.can_request_id_token);
        assert!(c.sdk_ready);
    }

    #[test]
    fn logged_out_line_can_browse_but_must_not_request_a_token() {
        let c = capabilities_for(HostProfile::LineIosLoggedOut);
        assert!(c.is_in_client, "still inside the LINE app");
        assert!(!c.is_logged_in);
        assert!(!c.can_request_id_token);
    }

    #[test]
    fn external_browser_must_not_look_like_line() {
        let c = capabilities_for(HostProfile::LineExternal);
        assert!(!c.is_in_client);
        assert!(!c.can_request_id_token);
        assert!(c.sdk_ready, "SDK loads fine; the host is simply not LINE");
    }

    #[test]
    fn revoked_consent_behaves_like_logged_out() {
        let revoked = capabilities_for(HostProfile::LineRevoked);
        let logged_out = capabilities_for(HostProfile::LineIosLoggedOut);
        assert_eq!(
            revoked.can_request_id_token,
            logged_out.can_request_id_token
        );
        assert!(revoked.notes.iter().any(|n| n.contains("revoked")));
    }

    #[test]
    fn sdk_failure_is_not_a_client_only_api_failure() {
        // The distinction matters: an SDK outage must degrade silently, while
        // an external-browser visit is a normal, non-degraded state.
        let offline = capabilities_for(HostProfile::SdkOffline);
        assert!(!offline.sdk_ready);
        assert!(offline
            .notes
            .iter()
            .any(|n| n.contains("WITHOUT redirecting")));
    }

    #[test]
    fn channel_secret_is_never_available_to_the_client_on_any_profile() {
        // CASE-SEC-002: structurally, no host can hand the client a secret.
        for profile in [
            HostProfile::LineIos,
            HostProfile::LineIosLoggedOut,
            HostProfile::LineAndroid,
            HostProfile::LineExternal,
            HostProfile::LineRevoked,
            HostProfile::PlainChrome,
            HostProfile::SdkOffline,
            HostProfile::SdkTimeout,
        ] {
            let api = api_availability(profile);
            assert_eq!(
                api.get("line.channelSecret"),
                Some(&false),
                "{profile:?} must not expose the channel secret"
            );
            assert_eq!(api.get("line.verifySignature"), Some(&false));
        }
    }

    #[test]
    fn send_messages_is_only_available_inside_line() {
        for (profile, expected) in [
            (HostProfile::LineIos, true),
            (HostProfile::LineAndroid, true),
            (HostProfile::LineExternal, false),
            (HostProfile::PlainChrome, false),
        ] {
            assert_eq!(
                api_availability(profile).get("liff.sendMessages"),
                Some(&expected),
                "{profile:?}"
            );
        }
    }

    #[test]
    fn get_profile_requires_a_logged_in_session() {
        for (profile, expected) in [
            (HostProfile::LineIos, true),
            (HostProfile::LineIosLoggedOut, false),
            (HostProfile::LineRevoked, false),
            (HostProfile::LineExternal, false),
        ] {
            assert_eq!(
                api_availability(profile).get("liff.getProfile"),
                Some(&expected),
                "{profile:?}"
            );
        }
    }

    #[test]
    fn line_hosts_reserve_top_and_bottom_insets_for_the_header() {
        let ios = insets_for(HostProfile::LineIos);
        assert!(ios["top"].as_i64().unwrap() > 0);
        assert!(ios["bottom"].as_i64().unwrap() > 0, "home indicator");
        let web = insets_for(HostProfile::PlainChrome);
        assert_eq!(web["top"], 0);
    }

    #[test]
    fn viewport_is_phone_sized_only_on_phone_hosts() {
        assert_eq!(viewport_for(HostProfile::LineIos)["width"], 375);
        assert_eq!(viewport_for(HostProfile::LineAndroid)["width"], 412);
        assert_eq!(viewport_for(HostProfile::PlainChrome)["width"], 1280);
    }

    #[test]
    fn user_agent_reflects_the_host_family() {
        assert!(ua_for(HostProfile::LineIos).contains("iPhone"));
        assert!(ua_for(HostProfile::LineAndroid).contains("Android"));
        assert!(ua_for(HostProfile::PlainChrome).contains("Windows"));
    }
}
