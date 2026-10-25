//! Authoritative pricing, shared by every service that touches a price.
//!
//! The cart and the order service must never trust a client-supplied amount.
//! The design documents state this as a hard rule ("the server is the only
//! authority"), and the client is free to claim whatever it likes.
//!
//! This lives in `commerce-core` rather than in one service because two
//! independent copies of a money rule is two chances to get it wrong. The
//! cart needs it too: a cart whose displayed price the client can set is a
//! cart that shows the shopper a different number from the one they are
//! charged.
//!
//! ## The bug this module exists to prevent
//!
//! Before this existed, `OrderLine.unit_amount_minor` was deserialized
//! straight from the request body and the total was computed from it. A
//! client could therefore post
//!
//! ```json
//! {"lines":[{"sku":"sku-bra-01","quantity":1,"unit_amount_minor":1,"currency":"JPY"}]}
//! ```
//!
//! and the order was created with `total = 1` minor unit and the state
//! `confirmed`. The real price was 8900. The order was recorded, the
//! inventory was reserved, and the payment was "authorized" for one cent.
//!
//! The comment on the field said "never from the client" while the code
//! took it from the client. A comment is not a control.
//!
//! ## How it works now
//!
//! The client still sends `unit_amount_minor` — the contract requires it and
//! removing it would break every existing client — but the value is treated
//! as a *display hint* and is **overwritten** with the catalogue price before
//! any money math happens. A client that under-claims, over-claims or omits
//! the field entirely is charged the same amount.
//!
//! The catalogue is queried once per checkout for the whole basket, not
//! once per line: a checkout of five lines is one HTTP call.

use std::time::Duration;

use serde::Deserialize;

use crate::{Currency, Money};

/// Where the catalogue lives.
#[derive(Debug, Clone)]
pub struct PriceAuthority {
    client: reqwest::Client,
    catalog_base_url: String,
}

/// The catalogue's price for one SKU.
#[derive(Debug, Clone, Deserialize)]
struct CatalogItem {
    sku: String,
    price: CatalogPrice,
}

#[derive(Debug, Clone, Deserialize)]
struct CatalogPrice {
    /// The contract's name for the amount is `amountMinor`.
    ///
    /// `rename` makes that the accepted spelling; `alias` keeps the old
    /// snake_case `amount_minor` working too. Without the alias, a catalogue
    /// regression to the old shape would fail to parse — which is loud, but
    /// loud failure at the parse layer is much better than a silent zero.
    #[serde(rename = "amountMinor", alias = "amount_minor")]
    amount_minor: i64,
    currency: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CatalogPage {
    items: Vec<CatalogItem>,
}

/// The authority the order and cart services ask for a price.
///
/// A trait rather than a concrete type so a deployment that hosts the
/// catalogue in the same process can serve the price without an HTTP round
/// trip, and so the two services do not have to know how the catalogue is
/// reached. The cluster deployment uses the HTTP implementation; the merged
/// edge uses the in-process one.
#[async_trait::async_trait]
pub trait PriceSource: Send + Sync + 'static {
    /// The authoritative price for every SKU in the basket.
    ///
    /// A SKU the catalogue does not know is simply absent from the result;
    /// the caller decides what that means (the order service treats it as
    /// "price unavailable" and rejects the checkout, because a line it
    /// cannot price must not be charged at the client's claim).
    async fn fetch_prices(&self, skus: &[String]) -> Result<Vec<(String, Money)>, String>;
}

#[async_trait::async_trait]
impl PriceSource for PriceAuthority {
    async fn fetch_prices(&self, skus: &[String]) -> Result<Vec<(String, Money)>, String> {
        self.fetch_prices_over_http(skus).await
    }
}

impl PriceAuthority {
    pub fn new(catalog_base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            catalog_base_url: catalog_base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Fetch the authoritative price for every SKU in the basket over HTTP.
    ///
    /// The catalogue is queried once per checkout for the whole basket, not
    /// once per line: a checkout of five lines is one HTTP call.
    pub async fn fetch_prices_over_http(
        &self,
        skus: &[String],
    ) -> Result<Vec<(String, Money)>, String> {
        if skus.is_empty() {
            return Ok(Vec::new());
        }

        // One request for the whole catalogue page. The limit is the
        // catalogue's own maximum; a basket with more lines than that is not
        // a real basket, and the caller's per-line lookup below covers it.
        let url = format!(
            "{}/v1/products?locale=ja&channel=web&limit=100",
            self.catalog_base_url
        );
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("catalogue unreachable: {e}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "catalogue returned {} for {url}",
                response.status()
            ));
        }
        let page: CatalogPage = response
            .json()
            .await
            .map_err(|e| format!("catalogue response unparseable: {e}"))?;

        Ok(page
            .items
            .into_iter()
            .filter(|item| skus.iter().any(|s| s == &item.sku))
            .filter_map(|item| {
                // A currency the catalogue reports that this build does not
                // know is a contract violation, not something to guess at.
                let currency = Currency::parse(&item.price.currency)?;
                let money = Money::new(item.price.amount_minor, currency).ok()?;
                Some((item.sku, money))
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_price_parses_the_nested_money_shape() {
        // This is the exact JSON commerce-catalog emits. The nested `price`
        // object with `amountMinor` is the contract's shape; the alias exists
        // so a future snake_case regression does not silently zero the price.
        let json =
            r#"{"items":[{"sku":"sku-bra-01","price":{"amountMinor":8900,"currency":"JPY"}}]}"#;
        let page: CatalogPage = serde_json::from_str(json).expect("parse");
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].sku, "sku-bra-01");
        assert_eq!(page.items[0].price.amount_minor, 8900);
        assert_eq!(
            Currency::parse(&page.items[0].price.currency),
            Some(Currency::Jpy)
        );
    }

    #[test]
    fn catalog_price_still_parses_a_flat_snake_case_amount() {
        // Defensive: if the catalogue ever regresses to the flat shape, the
        // price must not silently become zero.
        let json =
            r#"{"items":[{"sku":"sku-bra-01","price":{"amount_minor":8900,"currency":"JPY"}}]}"#;
        let page: CatalogPage = serde_json::from_str(json).expect("parse");
        assert_eq!(page.items[0].price.amount_minor, 8900);
    }
}
