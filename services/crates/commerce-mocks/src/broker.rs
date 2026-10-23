//! Event-broker mock — serves `CASE-IT-002`.
//!
//! The design says the relay is **at-least-once**, never exactly-once, and
//! that consumers dedupe via an Inbox key. This mock is built so a test can
//! *prove* both halves rather than take them on faith:
//!
//! - `__mock/redeliver` replays an already-delivered message, modelling the
//!   duplicate every at-least-once broker eventually produces.
//! - `__mock/inbox` exposes the consumer-side dedupe table, so a test asserts
//!   the consumer *saw* the duplicate but *processed* it once.
//!
//! If someone ever "fixes" the system by claiming exactly-once, this mock makes
//! the duplicate appear anyway.

use crate::fault::{Fault, FaultController, FaultMode};
use axum::{http::StatusCode, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub event_id: String,
    pub topic: String,
    pub aggregate_id: String,
    pub schema_version: u32,
    pub correlation_id: String,
    pub causation_id: Option<String>,
    pub occurred_at: String,
    pub payload: serde_json::Value,
}

impl Message {
    /// Schema-version mismatch is a *hard* error: a consumer that silently
    /// accepts an unknown version is how a bad deploy corrupts data.
    pub fn is_compatible(&self, consumer_version: u32) -> bool {
        self.schema_version == consumer_version
    }
}

/// The consumer-side dedupe record.
#[derive(Debug, Clone, Serialize)]
pub struct InboxEntry {
    pub event_id: String,
    pub topic: String,
    /// How many times this event was *delivered* to the consumer.
    pub delivery_count: u64,
    /// How many times it was actually *processed*. Must stay 1.
    pub processed_count: u64,
}

#[derive(Debug, Default)]
struct BrokerState {
    messages: Vec<Message>,
    inbox: HashMap<String, InboxEntry>,
    delivered: u64,
}

#[derive(Clone, Default)]
pub struct BrokerMock {
    state: Arc<RwLock<BrokerState>>,
    faults: FaultController,
}

impl BrokerMock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn faults(&self) -> FaultController {
        self.faults.clone()
    }

    /// Producer side. Records the message and returns whether it was accepted.
    ///
    /// A broker that is down must *reject* the write so the Outbox keeps the
    /// event unpublished — that is what makes the relay retriable.
    pub async fn publish(&self, msg: Message) -> Result<(), StatusCode> {
        if let Some(f) = self.faults.take("broker.publish") {
            if f.latency_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(f.latency_ms)).await;
            }
            let _ = f;
            return Err(StatusCode::from_u16(f.http_status)
                .unwrap_or(StatusCode::SERVICE_UNAVAILABLE));
        }
        let mut g = self.state.write().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        g.messages.push(msg);
        Ok(())
    }

    /// Consumer side. Delivers the message and applies Inbox dedupe.
    ///
    /// Returns `(delivered, processed)`: `delivered` counts the delivery
    /// attempt, `processed` counts real work. A duplicate must show
    /// `delivered == true` with no new processing.
    pub async fn consume(
        &self,
        event_id: &str,
        topic: &str,
        consumer_version: u32,
    ) -> Result<ConsumeOutcome, StatusCode> {
        if self.faults.take("broker.consume").is_some() {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }

        let mut g = self.state.write().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let msg = g
            .messages
            .iter()
            .find(|m| m.event_id == event_id)
            .cloned()
            .ok_or(StatusCode::NOT_FOUND)?;

        if !msg.is_compatible(consumer_version) {
            return Err(StatusCode::BAD_REQUEST);
        }

        g.delivered += 1;
        let entry = g
            .inbox
            .entry(event_id.to_string())
            .or_insert_with(|| InboxEntry {
                event_id: event_id.to_string(),
                topic: topic.to_string(),
                delivery_count: 0,
                processed_count: 0,
            });
        entry.delivery_count += 1;
        let is_duplicate = entry.processed_count > 0;
        if !is_duplicate {
            entry.processed_count += 1;
        }

        Ok(ConsumeOutcome {
            event_id: event_id.to_string(),
            delivered: true,
            processed: !is_duplicate,
            duplicate: is_duplicate,
            delivery_count: entry.delivery_count,
            processed_count: entry.processed_count,
        })
    }

    /// CASE-IT-002: force the broker to deliver an event again.
    pub async fn redeliver(&self, event_id: &str) -> Result<Json<serde_json::Value>, StatusCode> {
        let mut g = self.state.write().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let msg = g
            .messages
            .iter()
            .find(|m| m.event_id == event_id)
            .cloned()
            .ok_or(StatusCode::NOT_FOUND)?;
        let topic = msg.topic.clone();
        let outcome = {
            let entry = g.inbox.entry(event_id.to_string()).or_insert_with(|| InboxEntry {
                event_id: event_id.to_string(),
                topic: topic.clone(),
                delivery_count: 0,
                processed_count: 0,
            });
            entry.delivery_count += 1;
            entry.processed_count > 0
        };
        Ok(Json(serde_json::json!({
            "eventId": event_id,
            "redelivered": true,
            "duplicateDetectedByConsumer": outcome,
        })))
    }

    pub async fn inbox_dump(&self) -> Json<serde_json::Value> {
        let g = self.state.read().expect("broker lock");
        let mut v: Vec<_> = g.inbox.values().cloned().collect();
        v.sort_by(|a, b| a.event_id.cmp(&b.event_id));
        Json(serde_json::json!({
            "messages": g.messages.len(),
            "totalDeliveries": g.delivered,
            "inbox": v,
        }))
    }

    pub async fn reset(&self) {
        let mut g = self.state.write().expect("broker lock");
        g.messages.clear();
        g.inbox.clear();
        g.delivered = 0;
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConsumeOutcome {
    pub event_id: String,
    pub delivered: bool,
    pub processed: bool,
    pub duplicate: bool,
    pub delivery_count: u64,
    pub processed_count: u64,
}

#[derive(Debug, Deserialize)]
struct PublishBody {
    #[serde(flatten)]
    msg: Message,
}

#[derive(Debug, Deserialize)]
struct ConsumeBody {
    event_id: String,
    topic: String,
    #[serde(default = "one")]
    consumer_schema_version: u32,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Deserialize)]
struct RedeliverBody {
    event_id: String,
}

pub fn router(mock: BrokerMock) -> Router {
    Router::new()
        .route(
            "/v1/broker/publish",
            post({
                let m = mock.clone();
                move |Json(b): Json<PublishBody>| {
                    let m = m.clone();
                    async move {
                        match m.publish(b.msg).await {
                            Ok(()) => (StatusCode::ACCEPTED, "accepted"),
                            Err(s) => (s, "rejected"),
                        }
                    }
                }
            }),
        )
        .route(
            "/v1/broker/consume",
            post({
                let m = mock.clone();
                move |Json(b): Json<ConsumeBody>| {
                    let m = m.clone();
                    async move {
                        match m
                            .consume(&b.event_id, &b.topic, b.consumer_schema_version)
                            .await
                        {
                            Ok(o) => (
                                StatusCode::OK,
                                Json(serde_json::to_value(o).unwrap_or(serde_json::Value::Null)),
                            ),
                            Err(s) => (
                                s,
                                Json(serde_json::json!({ "error": s.as_u16() })),
                            ),
                        }
                    }
                }
            }),
        )
        .route(
            "/v1/broker/redeliver",
            post({
                let m = mock.clone();
                move |Json(b): Json<RedeliverBody>| {
                    let m = m.clone();
                    async move { m.redeliver(&b.event_id).await }
                }
            }),
        )
        .route(
            "/__mock/inbox",
            get({
                let m = mock.clone();
                move || {
                    let m = m.clone();
                    async move { m.inbox_dump().await }
                }
            }),
        )
        .route(
            "/__mock/reset",
            post({
                let m = mock.clone();
                move || {
                    let m = m.clone();
                    async move {
                        m.reset().await;
                        Json(serde_json::json!({ "ok": true }))
                    }
                }
            }),
        )
}

/// Re-exported so tests can build a fault without importing `crate::fault`.
pub fn fault(target: &str, mode: FaultMode, count: u32) -> Fault {
    Fault {
        target: target.to_string(),
        mode,
        count,
        latency_ms: 0,
        http_status: 503,
        drop_response: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(id: &str) -> Message {
        Message {
            event_id: id.into(),
            topic: "order.confirmed".into(),
            aggregate_id: "ord-1".into(),
            schema_version: 1,
            correlation_id: "corr-1".into(),
            causation_id: None,
            occurred_at: "2026-10-17T00:00:00Z".into(),
            payload: serde_json::json!({"orderId": "ord-1"}),
        }
    }

    #[tokio::test]
    async fn publish_then_consume_processes_once() {
        let b = BrokerMock::new();
        b.publish(msg("e1")).await.unwrap();
        let o = b.consume("e1", "order.confirmed", 1).await.unwrap();
        assert!(o.processed);
        assert!(!o.duplicate);
        assert_eq!(o.processed_count, 1);
    }

    #[tokio::test]
    async fn redelivery_is_delivered_but_not_reprocessed() {
        // This is the at-least-once contract in one test.
        let b = BrokerMock::new();
        b.publish(msg("e2")).await.unwrap();
        let first = b.consume("e2", "order.confirmed", 1).await.unwrap();
        assert!(first.processed);

        let _ = b.redeliver("e2").await.unwrap();

        let second = b.consume("e2", "order.confirmed", 1).await.unwrap();
        assert!(second.delivered, "the duplicate WAS delivered");
        assert!(!second.processed, "but was NOT reprocessed");
        assert!(second.duplicate);
        assert_eq!(second.delivery_count, 3, "1 original + 1 redelivery + 1 consume");
        assert_eq!(second.processed_count, 1, "processing happened exactly once");
    }

    #[tokio::test]
    async fn schema_version_mismatch_is_rejected_not_silently_accepted() {
        let b = BrokerMock::new();
        b.publish(msg("e3")).await.unwrap();
        let err = b.consume("e3", "order.confirmed", 99).await.unwrap_err();
        assert_eq!(err, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn consuming_an_unknown_event_is_404() {
        let b = BrokerMock::new();
        assert_eq!(
            b.consume("nope", "t", 1).await.unwrap_err(),
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn broker_down_rejects_publish_so_the_outbox_keeps_the_event() {
        // CASE-CHAOS-002: the producer must learn it failed, or the Outbox row
        // would be marked published and the event lost forever.
        let b = BrokerMock::new();
        b.faults().inject(fault("broker.publish", FaultMode::NextN, 2));

        assert!(b.publish(msg("e4")).await.is_err());
        assert!(b.publish(msg("e5")).await.is_err());
        // Outage over — the relay retries and succeeds.
        b.publish(msg("e4")).await.unwrap();

        let dump = b.inbox_dump().await;
        assert_eq!(dump.0["messages"], 1, "only the retried event landed");
    }

    #[tokio::test]
    async fn inbox_dump_shows_delivery_and_process_divergence() {
        let b = BrokerMock::new();
        b.publish(msg("e6")).await.unwrap();
        b.consume("e6", "t", 1).await.unwrap();
        let _ = b.redeliver("e6").await.unwrap();
        b.consume("e6", "t", 1).await.unwrap();

        let dump = b.inbox_dump().await;
        let entry = &dump.0["inbox"][0];
        assert!(entry["delivery_count"].as_u64().unwrap() > entry["processed_count"].as_u64().unwrap());
    }

    #[tokio::test]
    async fn reset_clears_messages_and_inbox() {
        let b = BrokerMock::new();
        b.publish(msg("e7")).await.unwrap();
        b.consume("e7", "t", 1).await.unwrap();
        b.reset().await;
        let dump = b.inbox_dump().await;
        assert_eq!(dump.0["messages"], 0);
        assert_eq!(dump.0["inbox"].as_array().unwrap().len(), 0);
    }
}
