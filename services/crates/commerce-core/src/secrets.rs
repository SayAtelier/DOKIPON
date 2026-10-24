//! The credentials a deployment must set, and the development values it must
//! not keep.
//!
//! ## Why this module exists
//!
//! The development credentials are published in this repository. A deployment
//! that keeps one of them is open to anyone who has read the source:
//!
//! - the admin facade answers to `admin` with the development password, so
//!   the operator console — orders, inventory, reconciliation — is reachable;
//! - the identity service accepts an id token signed with the development
//!   key, so anyone can mint a session for any user;
//! - the payment adapter accepts a webhook signed with the development
//!   secret, so anyone can confirm a payment.
//!
//! A default that is only a default in the source is a default everywhere
//! else too: a deployment that forgets to set the variable gets it, and
//! nothing fails. The failure is silent, which is the worst kind — the
//! service answers normally, it just answers to everybody.
//!
//! So the values are refused here, at the entry point, before the service
//! that needs them starts. A deployment that has not set a credential does
//! not come up at all; it does not come up insecurely.
//!
//! ## The escape hatch
//!
//! [`ALLOW_DEV_SECRETS`] accepts the development values. It exists for a
//! local run, and it is never set by a deployment.

/// The environment variable that says a deployment accepts the development
/// credentials.
pub const ALLOW_DEV_SECRETS: &str = "DOKIPON_ALLOW_DEV_SECRETS";

/// The development admin password.
pub const DEV_ADMIN_PASSWORD: &str = "dev-only-admin";

/// The development LINE id token signing key.
pub const DEV_LINE_ID_TOKEN_KEY: &str = "dev-only-key";

/// The development LINE channel id.
pub const DEV_LINE_CHANNEL_ID: &str = "dev-channel";

/// The development PSP webhook secret.
pub const DEV_PSP_WEBHOOK_SECRET: &str = "dev-only-secret";

/// Read a credential that a deployment must provide.
///
/// The value is returned, so the caller uses the very value it checked.
///
/// A credential that is missing, empty, or still at its development value is
/// refused. The development values are in this repository, so a deployment
/// that keeps one is open to anyone who has read it.
pub fn require(name: &str, dev_default: &str) -> Result<String, String> {
    match std::env::var(name) {
        Ok(value) if value.trim().is_empty() => Err(format!("{name} must not be empty.")),
        Ok(value) => {
            if value == dev_default && !dev_allowed() {
                return Err(format!(
                    "{name} still holds its development value. That value is published in this \
                     repository, so anyone who has read it can reach this deployment. Set {name} \
                     to a real value, or set {ALLOW_DEV_SECRETS}=1 for a local run."
                ));
            }
            Ok(value)
        }
        Err(_) => Err(format!(
            "{name} must be set. Set it to a real value, or set {ALLOW_DEV_SECRETS}=1 for a local \
             run."
        )),
    }
}

/// Whether the deployment has said it accepts the development credentials.
fn dev_allowed() -> bool {
    matches!(
        std::env::var(ALLOW_DEV_SECRETS).ok().as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

/// The services whose credentials a deployment has to set.
///
/// The services are named here so that an entry point asks for exactly the
/// credentials its own service needs. A gateway, which only forwards, needs
/// none; the in-process edge, which serves every service, needs all of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Service {
    /// The operator console's facade.
    Admin,
    /// The sessions: the guest session and the LINE id token exchange.
    Identity,
    /// The payment adapter.
    Payment,
    /// Every service — the in-process edge serves all of them.
    All,
}

impl Service {
    /// The credentials this service needs: the variable's name, and the
    /// development value it must not keep.
    fn credentials(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Service::Admin => &[("ADMIN_PASSWORD", DEV_ADMIN_PASSWORD)],
            Service::Identity => &[
                ("LINE_ID_TOKEN_KEY", DEV_LINE_ID_TOKEN_KEY),
                ("LINE_CHANNEL_ID", DEV_LINE_CHANNEL_ID),
            ],
            Service::Payment => &[("PSP_WEBHOOK_SECRET", DEV_PSP_WEBHOOK_SECRET)],
            Service::All => &[
                ("ADMIN_PASSWORD", DEV_ADMIN_PASSWORD),
                ("LINE_ID_TOKEN_KEY", DEV_LINE_ID_TOKEN_KEY),
                ("LINE_CHANNEL_ID", DEV_LINE_CHANNEL_ID),
                ("PSP_WEBHOOK_SECRET", DEV_PSP_WEBHOOK_SECRET),
            ],
        }
    }
}

/// Check the credentials `service` needs.
///
/// A deployment calls this before it starts the service, so a deployment that
/// has not set a credential does not come up at all — it does not come up
/// insecurely. The services read the same variables, so the value that was
/// checked is the value they use.
pub fn require_for(service: Service) -> Result<(), String> {
    for (name, dev_default) in service.credentials() {
        require(name, dev_default)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    /// The environment is process-wide and the tests in one binary run in
    /// parallel, so every case that changes it takes this lock and puts the
    /// environment back the way it found it.
    static ENV: Mutex<()> = Mutex::new(());

    fn lock() -> MutexGuard<'static, ()> {
        ENV.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Run `body` with `vars` set, then restore the environment.
    fn with_env<F: FnOnce()>(vars: &[(&str, &str)], body: F) {
        let saved: Vec<(String, Option<String>)> = vars
            .iter()
            .map(|(name, _)| (name.to_string(), std::env::var(name).ok()))
            .collect();
        for (name, value) in vars {
            std::env::set_var(name, value);
        }
        body();
        for (name, value) in saved {
            match value {
                Some(value) => std::env::set_var(&name, value),
                None => std::env::remove_var(&name),
            }
        }
    }

    #[test]
    fn a_real_credential_is_returned() {
        let _guard = lock();
        with_env(&[("DOKIPON_TEST_SECRET", "a-real-value")], || {
            let value = require("DOKIPON_TEST_SECRET", DEV_ADMIN_PASSWORD).unwrap();
            assert_eq!(value, "a-real-value");
        });
    }

    #[test]
    fn a_missing_credential_is_refused() {
        let _guard = lock();
        std::env::remove_var("DOKIPON_TEST_ABSENT");
        let error = require("DOKIPON_TEST_ABSENT", DEV_ADMIN_PASSWORD).unwrap_err();
        assert!(error.contains("must be set"), "{error}");
    }

    #[test]
    fn an_empty_credential_is_refused() {
        let _guard = lock();
        with_env(&[("DOKIPON_TEST_EMPTY", "")], || {
            let error = require("DOKIPON_TEST_EMPTY", DEV_ADMIN_PASSWORD).unwrap_err();
            assert!(error.contains("must not be empty"), "{error}");
        });
    }

    #[test]
    fn the_development_value_is_refused() {
        let _guard = lock();
        with_env(&[("DOKIPON_TEST_DEV", DEV_ADMIN_PASSWORD)], || {
            std::env::remove_var(ALLOW_DEV_SECRETS);
            let error = require("DOKIPON_TEST_DEV", DEV_ADMIN_PASSWORD).unwrap_err();
            assert!(error.contains("development value"), "{error}");
        });
    }

    #[test]
    fn the_escape_hatch_accepts_the_development_value() {
        let _guard = lock();
        with_env(
            &[
                ("DOKIPON_TEST_DEV2", DEV_PSP_WEBHOOK_SECRET),
                (ALLOW_DEV_SECRETS, "1"),
            ],
            || {
                let value = require("DOKIPON_TEST_DEV2", DEV_PSP_WEBHOOK_SECRET).unwrap();
                assert_eq!(value, DEV_PSP_WEBHOOK_SECRET);
            },
        );
    }

    #[test]
    fn the_development_values_are_the_ones_the_services_default_to() {
        // The point of the constants is that they are the values the services
        // would use if nothing checked them. If one of these changes and the
        // service's own default does not, the guard stops guarding.
        assert_eq!(DEV_ADMIN_PASSWORD, "dev-only-admin");
        assert_eq!(DEV_LINE_ID_TOKEN_KEY, "dev-only-key");
        assert_eq!(DEV_LINE_CHANNEL_ID, "dev-channel");
        assert_eq!(DEV_PSP_WEBHOOK_SECRET, "dev-only-secret");
    }

    #[test]
    fn each_service_asks_for_exactly_its_own_credentials() {
        let _guard = lock();
        std::env::remove_var(ALLOW_DEV_SECRETS);
        for name in [
            "ADMIN_PASSWORD",
            "LINE_ID_TOKEN_KEY",
            "LINE_CHANNEL_ID",
            "PSP_WEBHOOK_SECRET",
        ] {
            std::env::remove_var(name);
        }

        // With nothing set, every service is refused, and the error names
        // the variable that is missing — so the deployment says which one to
        // set.
        let error = require_for(Service::Admin).unwrap_err();
        assert!(error.contains("ADMIN_PASSWORD"), "{error}");
        let error = require_for(Service::Identity).unwrap_err();
        assert!(error.contains("LINE_ID_TOKEN_KEY"), "{error}");
        let error = require_for(Service::Payment).unwrap_err();
        assert!(error.contains("PSP_WEBHOOK_SECRET"), "{error}");
        let error = require_for(Service::All).unwrap_err();
        assert!(error.contains("ADMIN_PASSWORD"), "{error}");

        // The admin service does not ask for the identity service's key:
        // setting the admin password is enough for it to start.
        std::env::set_var("ADMIN_PASSWORD", "a-real-password");
        require_for(Service::Admin).expect("the admin service starts");
        let error = require_for(Service::All).unwrap_err();
        assert!(error.contains("LINE_ID_TOKEN_KEY"), "{error}");

        for name in [
            "ADMIN_PASSWORD",
            "LINE_ID_TOKEN_KEY",
            "LINE_CHANNEL_ID",
            "PSP_WEBHOOK_SECRET",
        ] {
            std::env::remove_var(name);
        }
    }
}
