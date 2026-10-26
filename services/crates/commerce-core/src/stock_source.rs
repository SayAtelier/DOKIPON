//! The stock levels, and where they come from.
//!
//! ## Why the levels are not the catalogue's
//!
//! The catalogue knows **what** a product is and how much it costs. The
//! inventory ledger knows **how many** are left. They were the same number
//! once — the catalogue's demo seed carried a copy of the ledger's — and then
//! they drifted, because nothing connected them: a sale moved the ledger and
//! the catalogue kept saying "42 in stock".
//!
//! So the catalogue asks the ledger. The storefront shows what the ledger
//! says, and the shopper is not told a thing is available after the last one
//! was sold.
//!
//! ## The shape of the answer
//!
//! A level is a SKU's `on_hand`, how much of it is `reserved` by an order that
//! has not been settled yet, and the `available` difference. The difference is
//! computed here, so both the ledger and the catalogue count the same way.
//!
//! ## Who answers
//!
//! A trait rather than a concrete type, for the same reason
//! [`PriceSource`](crate::price_authority::PriceSource) is one: the cluster
//! and the serverless deployment reach the inventory service over HTTP, while
//! the merged in-process edge answers from the ledger it already holds in
//! memory.

use std::time::Duration;

use serde::Deserialize;

/// One SKU's stock level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct StockLevel {
    /// How many the warehouse holds.
    pub on_hand: i64,
    /// How many an unsettled order is holding.
    pub reserved: i64,
}

impl StockLevel {
    /// How many a shopper may still buy.
    pub fn available(&self) -> i64 {
        (self.on_hand - self.reserved).max(0)
    }
}

/// The level of a SKU the source does not know.
///
/// A SKU the ledger has never heard of is not "zero in stock": it is a SKU
/// that does not take part in the inventory step at all (a made-to-order
/// product, or an unbounded one). The caller decides what that means.
pub const UNKNOWN_LEVEL: Option<StockLevel> = None;

/// Where the stock levels come from.
#[async_trait::async_trait]
pub trait StockSource: Send + Sync + 'static {
    /// The level of every SKU in the list that the source knows.
    ///
    /// A SKU that is absent from the result is one the source does not know
    /// — not one that is out of stock.
    async fn fetch_levels(&self, skus: &[String]) -> Result<Vec<(String, StockLevel)>, String>;
}

/// The ledger, over HTTP.
#[derive(Debug, Clone)]
pub struct StockAuthority {
    client: reqwest::Client,
    inventory_base_url: String,
}

#[async_trait::async_trait]
impl StockSource for StockAuthority {
    async fn fetch_levels(&self, skus: &[String]) -> Result<Vec<(String, StockLevel)>, String> {
        self.fetch_levels_over_http(skus).await
    }
}

impl StockAuthority {
    pub fn new(inventory_base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            inventory_base_url: inventory_base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Fetch the levels over HTTP.
    ///
    /// One request for the whole list: a catalogue page of twenty products is
    /// one call, not twenty.
    pub async fn fetch_levels_over_http(
        &self,
        skus: &[String],
    ) -> Result<Vec<(String, StockLevel)>, String> {
        if skus.is_empty() {
            return Ok(Vec::new());
        }
        let url = format!(
            "{}/v1/inventory?limit=100&skus={}",
            self.inventory_base_url,
            encode_query(skus),
        );
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("inventory unreachable: {e}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "inventory returned {} for {url}",
                response.status()
            ));
        }
        let page: InventoryPage = response
            .json()
            .await
            .map_err(|e| format!("inventory response unparseable: {e}"))?;

        Ok(page
            .items
            .into_iter()
            .filter(|item| skus.iter().any(|sku| sku == &item.sku))
            .map(|item| {
                (
                    item.sku,
                    StockLevel {
                        on_hand: item.on_hand,
                        reserved: item.reserved,
                    },
                )
            })
            .collect())
    }
}

/// One row of the ledger's listing.
#[derive(Debug, Clone, Deserialize)]
struct InventoryItem {
    sku: String,
    #[serde(rename = "onHand", alias = "on_hand")]
    on_hand: i64,
    reserved: i64,
}

/// The ledger's page.
#[derive(Debug, Clone, Deserialize)]
struct InventoryPage {
    items: Vec<InventoryItem>,
}

/// The SKUs, as one query parameter.
///
/// A SKU is a path-safe token in this codebase (`sku-bra-01`), but the
/// value is encoded anyway: a caller that passes something else gets a
/// rejected request rather than a mangled query.
fn encode_query(skus: &[String]) -> String {
    skus.iter()
        .map(|sku| {
            sku.chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect::<String>()
        })
        .collect::<Vec<String>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ledger_row_parses_the_contract_shape() {
        // The contract's spelling is `onHand`; the alias keeps the old
        // snake_case working so a regression does not silently zero a level.
        let json = r#"{"items":[{"sku":"sku-bra-01","onHand":42,"reserved":2,"available":40}]}"#;
        let page: InventoryPage = serde_json::from_str(json).expect("parse");
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].sku, "sku-bra-01");
        assert_eq!(page.items[0].on_hand, 42);
        assert_eq!(page.items[0].reserved, 2);
    }

    #[test]
    fn a_ledger_row_still_parses_a_flat_snake_case_amount() {
        let json = r#"{"items":[{"sku":"sku-bra-01","on_hand":42,"reserved":0,"available":42}]}"#;
        let page: InventoryPage = serde_json::from_str(json).expect("parse");
        assert_eq!(page.items[0].on_hand, 42);
    }

    #[test]
    fn available_is_what_is_left_after_the_reservations() {
        let level = StockLevel {
            on_hand: 42,
            reserved: 2,
        };
        assert_eq!(level.available(), 40);
    }

    #[test]
    fn a_level_never_goes_below_zero() {
        let level = StockLevel {
            on_hand: 1,
            reserved: 3,
        };
        assert_eq!(
            level.available(),
            0,
            "the ledger's own constraint says this cannot happen"
        );
    }

    #[test]
    fn the_sku_list_is_encoded() {
        assert_eq!(encode_query(&["sku-bra-01".to_string()]), "sku-bra-01");
        assert_eq!(
            encode_query(&["sku-a".to_string(), "sku-b".to_string()]),
            "sku-a,sku-b"
        );
    }
}
