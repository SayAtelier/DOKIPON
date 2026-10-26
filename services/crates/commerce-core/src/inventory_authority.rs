//! Stock reservations, and the service that holds them.
//!
//! ## Why the order service asks
//!
//! The order service's Saga has a step called "reserve stock", and until
//! now it did not reserve anything: it **allocated a reservation id** and
//! moved on. The ledger never learned that the units were spoken for, so the
//! same last bra could be sold twice, and the operator's stock view was a
//! number that never moved.
//!
//! Now the step is real. The order service asks the inventory service to
//! reserve, and the id it gets back is the id in the ledger. A payment that
//! is declined releases the reservation; a confirmed order commits it, and
//! only a commit takes the units out of the warehouse.
//!
//! ## Failing closed
//!
//! If the ledger cannot be reached, the checkout is **refused**. A shop that
//! cannot reserve what it is about to sell must not sell it: the alternative
//! is a sale with no reservation behind it, and the ledger would be wrong
//! about what is left.
//!
//! ## The two implementations
//!
//! A trait, like [`PriceSource`](crate::price_authority::PriceSource): the
//! cluster and the serverless deployment reach the inventory service over
//! HTTP; the merged in-process edge answers from the ledger it already holds.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::ReservationState;

/// A stock reservation.
///
/// The same shape the inventory service keeps in its ledger, so the two do
/// not have to be translated into each other at every boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reservation {
    pub reservation_id: String,
    pub sku: String,
    pub order_id: String,
    pub quantity: i64,
    pub state: ReservationState,
    pub expires_at_ms: i64,
}

/// Where the reservations come from.
#[async_trait::async_trait]
pub trait ReservationSource: Send + Sync + 'static {
    /// Reserve `quantity` of `sku` for `order_id`.
    ///
    /// Reserving twice for the same order and SKU returns the **same**
    /// reservation: the ledger decides, so two requests cannot both hold the
    /// units.
    async fn reserve(
        &self,
        sku: &str,
        order_id: &str,
        quantity: i64,
    ) -> Result<Reservation, String>;

    /// Take the units out of the warehouse: the payment succeeded.
    async fn commit(&self, reservation_id: &str) -> Result<Reservation, String>;

    /// Give the units back: the payment failed, or the order was rejected.
    async fn release(&self, reservation_id: &str) -> Result<Reservation, String>;

    /// The reservations this order holds.
    ///
    /// The ledger is the authority on what an order holds: an order with
    /// several lines has several reservations, and the order service asks
    /// for them when it settles the order later — at reconciliation time,
    /// when the request that created them is long gone.
    async fn list_for_order(&self, order_id: &str) -> Result<Vec<Reservation>, String>;
}

/// The inventory service, over HTTP.
#[derive(Debug, Clone)]
pub struct InventoryAuthority {
    client: reqwest::Client,
    inventory_base_url: String,
}

#[async_trait::async_trait]
impl ReservationSource for InventoryAuthority {
    async fn reserve(
        &self,
        sku: &str,
        order_id: &str,
        quantity: i64,
    ) -> Result<Reservation, String> {
        self.reserve_over_http(sku, order_id, quantity).await
    }

    async fn commit(&self, reservation_id: &str) -> Result<Reservation, String> {
        self.post(
            &format!("/v1/inventory/reservations/{reservation_id}/commit"),
            None,
        )
        .await
    }

    async fn release(&self, reservation_id: &str) -> Result<Reservation, String> {
        self.post(
            &format!("/v1/inventory/reservations/{reservation_id}/release"),
            None,
        )
        .await
    }

    async fn list_for_order(&self, order_id: &str) -> Result<Vec<Reservation>, String> {
        let url = format!(
            "{}/v1/inventory/reservations?order_id={}",
            self.inventory_base_url, order_id
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
        response
            .json()
            .await
            .map_err(|e| format!("inventory response unparseable: {e}"))
    }
}

impl InventoryAuthority {
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

    /// Reserve over HTTP.
    pub async fn reserve_over_http(
        &self,
        sku: &str,
        order_id: &str,
        quantity: i64,
    ) -> Result<Reservation, String> {
        let body = serde_json::json!({
            "sku": sku,
            "order_id": order_id,
            "quantity": quantity,
        });
        self.post("/v1/inventory/reservations", Some(body)).await
    }

    async fn post(
        &self,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<Reservation, String> {
        let url = format!("{}{path}", self.inventory_base_url);
        let mut request = self.client.post(&url);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request
            .send()
            .await
            .map_err(|e| format!("inventory unreachable: {e}"))?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("inventory returned {status}: {body}"));
        }
        response
            .json()
            .await
            .map_err(|e| format!("inventory response unparseable: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reservation_parses_the_contract_shape() {
        let json = r#"{"reservation_id":"res-000001","sku":"sku-bra-01","order_id":"ord-000001","quantity":2,"state":"reserved","expires_at_ms":1700000000000}"#;
        let reservation: Reservation = serde_json::from_str(json).expect("parse");
        assert_eq!(reservation.reservation_id, "res-000001");
        assert_eq!(reservation.sku, "sku-bra-01");
        assert_eq!(reservation.order_id, "ord-000001");
        assert_eq!(reservation.quantity, 2);
        assert_eq!(reservation.state, ReservationState::Reserved);
    }

    #[test]
    fn the_ledger_speaks_snake_case() {
        // The inventory service's own shape is snake_case (`reservation_id`),
        // while the catalogue's is camelCase. A round trip must not need a
        // rename on either side.
        let json = r#"{"reservation_id":"res-000002","sku":"sku-a","order_id":"ord-2","quantity":1,"state":"committed","expires_at_ms":1700000000000}"#;
        let reservation: Reservation = serde_json::from_str(json).expect("parse");
        assert_eq!(reservation.reservation_id, "res-000002");
        assert_eq!(reservation.state, ReservationState::Committed);
        assert_eq!(reservation.expires_at_ms, 1_700_000_000_000);
    }
}
