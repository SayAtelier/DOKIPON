//! The wire format matches the contract, field by field.
//!
//! ## Why this test exists
//!
//! `contracts/openapi.yaml` is the contract of record, and the Dart client
//! reads its field names literally — `json['productId']`, `json['amountMinor']`,
//! `json['sessionId']` — with no fallback for the snake_case spelling in most
//! cases. A server that emits `product_id` leaves the client with an empty
//! string; one that emits `amount_minor` renders every price as 0.
//!
//! The contract is not uniformly camelCase, and that is the point of this
//! test: `amountMinor` and `sessionId` are camelCase, while
//! `unit_amount_snapshot`, `next_cursor`, `owner_id` and `expires_at_ms` are
//! snake_case. A blanket rename to camelCase breaks the second group just as
//! surely as the current snake_case breaks the first, so every assertion
//! below names the exact spelling the contract uses.
//!
//! No network and no ports: the gateway is driven in-process with
//! `tower::ServiceExt::oneshot`.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

/// The session the requests act as.
const SESSION: &str = "sess-000001";

/// An idempotency key, in the contract's UUID format.
const KEY: &str = "11111111-1111-1111-1111-111111111111";

/// The gateway under test.
///
/// One gateway is built per test and cloned per request: the services keep
/// their state in memory, so a checkout created by one request must still be
/// there for the next one.
fn gateway() -> Router {
    let admin = commerce_admin::AdminState::from_env()
        .expect("the admin facade builds without a database");
    let (edge, _services) = commerce_edge::build(admin);
    edge
}

async fn call(
    gateway: &Router,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let payload = match body {
        Some(value) => Body::from(value.to_string()),
        None => Body::empty(),
    };
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("x-session-id", SESSION)
        .header("idempotency-key", KEY)
        .header("x-channel", "web")
        .header("content-type", "application/json")
        .body(payload)
        .expect("the request builds");
    let response = gateway
        .clone()
        .oneshot(request)
        .await
        .expect("the gateway answers");
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

#[tokio::test]
async fn the_product_list_uses_the_contract_field_names() {
    let gateway = gateway();
    let (status, body) = call(&gateway, "GET", "/v1/products", None).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    let items = body["items"].as_array().expect("items is an array");
    assert!(!items.is_empty(), "the seeded catalogue is not empty");
    let first = &items[0];

    // `ProductSummary`: productId, sku, title, price (Money), availability.
    assert!(first["productId"].is_string(), "productId: {first}");
    assert!(first["sku"].is_string(), "sku: {first}");
    assert!(first["title"].is_string(), "title: {first}");
    assert!(first["availability"].is_string(), "availability: {first}");

    // `Money` is nested and camelCase.
    assert!(
        first["price"]["amountMinor"].is_i64(),
        "price.amountMinor: {first}"
    );
    assert!(first["price"]["currency"].is_string(), "price.currency: {first}");

    // The old snake_case spelling must be gone: the client reads `productId`
    // and `price.amountMinor` only.
    assert!(first.get("product_id").is_none(), "{first}");
    assert!(first.get("amount_minor").is_none(), "{first}");

    // `ProductPage.next_cursor` is snake_case in the contract — the one field
    // of this page that is.
    assert!(
        body.as_object().expect("the page is an object").contains_key("next_cursor"),
        "next_cursor: {body}"
    );
    assert!(body.get("nextCursor").is_none(), "{body}");
}

#[tokio::test]
async fn the_product_detail_uses_the_contract_field_names() {
    let gateway = gateway();
    let (status, body) = call(&gateway, "GET", "/v1/products/p-bra-01", None).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    // `ProductDetail`.
    assert_eq!(body["productId"], "p-bra-01");
    assert!(body["sku"].is_string(), "sku: {body}");
    assert!(body["titleI18n"].is_object(), "titleI18n: {body}");
    assert!(body["descriptionI18n"].is_object(), "descriptionI18n: {body}");
    assert!(body["categoryId"].is_string(), "categoryId: {body}");
    assert!(body["published"].is_boolean(), "published: {body}");
    assert!(body["version"].is_i64(), "version: {body}");
    assert!(body["stockPolicy"].is_string(), "stockPolicy: {body}");
    assert!(body["availability"].is_string(), "availability: {body}");

    // `Money` nested, camelCase.
    assert!(body["price"]["amountMinor"].is_i64(), "price: {body}");
    assert!(body["price"]["currency"].is_string(), "price: {body}");

    // `ChannelEligibility` is snake_case in the contract — `line_mini_app`,
    // not `lineMiniApp`.
    assert!(body["channelEligibility"]["web"].is_boolean(), "{body}");
    assert!(body["channelEligibility"]["app"].is_boolean(), "{body}");
    assert!(
        body["channelEligibility"]["line_mini_app"].is_boolean(),
        "channelEligibility: {body}"
    );

    // The old snake_case spellings must be gone.
    for gone in ["product_id", "title_i18n", "category_id", "stock_policy"] {
        assert!(body.get(gone).is_none(), "{gone} is still emitted: {body}");
    }
}

#[tokio::test]
async fn a_guest_session_uses_the_contract_field_names() {
    let gateway = gateway();
    let (status, body) = call(
        &gateway,
        "POST",
        "/v1/sessions/guest",
        Some(json!({"channel": "line"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    // `Session`: sessionId, kind, channel, expires_at_ms.
    assert!(body["sessionId"].is_string(), "sessionId: {body}");
    assert_eq!(body["kind"], "guest", "kind: {body}");
    assert_eq!(body["channel"], "line", "channel: {body}");
    assert!(body["expires_at_ms"].is_i64(), "expires_at_ms: {body}");

    // The old snake_case spelling must be gone.
    assert!(body.get("session_id").is_none(), "{body}");

    // The channel the contract spells `line` — not `line_mini_app`.
    assert!(body.get("line_mini_app").is_none(), "{body}");
}

#[tokio::test]
async fn a_line_session_is_validated_by_the_identity_service() {
    // A malformed id_token must be refused with the identity service's own
    // problem code. A stub that minted a session locally would answer 200.
    let gateway = gateway();
    let (status, body) = call(
        &gateway,
        "POST",
        "/v1/sessions/line",
        Some(json!({"id_token": "not-a-jwt", "nonce": "n-1", "channel": "line"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body: {body}");
    assert_eq!(body["code"], "identity.malformed_token", "body: {body}");
}

#[tokio::test]
async fn a_checkout_and_its_order_use_the_contract_field_names() {
    // The contract's `CheckoutRequest`: `lines` and `paymentMethodRef`. The
    // line's `unit_amount_minor` is a display hint the server discards — the
    // catalogue price wins.
    let gateway = gateway();
    let (status, body) = call(
        &gateway,
        "POST",
        "/v1/checkouts",
        Some(json!({
            "lines": [{"sku": "sku-bra-01", "quantity": 2, "unit_amount_minor": 1}],
            "paymentMethodRef": "tok_test",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");

    // `CheckoutSnapshot`: checkoutId, status. The contract says `201` when
    // the order reached `confirmed` and `202` while it is still processing.
    assert!(body["checkoutId"].is_string(), "checkoutId: {body}");
    assert!(body["status"].is_string(), "status: {body}");
    let checkout_id = body["checkoutId"].as_str().unwrap().to_string();

    let (status, body) = call(&gateway, "GET", &format!("/v1/checkouts/{checkout_id}"), None).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["checkoutId"], checkout_id.as_str(), "checkoutId: {body}");
    assert!(body["orderId"].is_string(), "orderId: {body}");
    assert!(body["status"].is_string(), "status: {body}");
    assert!(body["terminal"].is_boolean(), "terminal: {body}");
    assert!(
        body["awaitingReconciliation"].is_boolean(),
        "awaitingReconciliation: {body}"
    );
    let order_id = body["orderId"].as_str().unwrap().to_string();

    let (status, body) = call(&gateway, "GET", &format!("/v1/orders/{order_id}"), None).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    // `OrderDetail`: orderId, state, lines, total, reservationId,
    // awaitingReconciliation, timeline.
    assert_eq!(body["orderId"], order_id.as_str(), "orderId: {body}");
    assert!(body["state"].is_string(), "state: {body}");
    assert!(body["reservationId"].is_string(), "reservationId: {body}");
    assert!(
        body["awaitingReconciliation"].is_boolean(),
        "awaitingReconciliation: {body}"
    );

    // `OrderDetail.lines[]`: sku, quantity, unitPrice — and `unitPrice` is a
    // `Money` object, the same shape as the order's `total` and the
    // product's `price`. The line used to carry a flat `unit_amount_minor`,
    // which is what no client reads: the Dart client looks for `unitAmount`,
    // `unit_amount`, `unitPrice` or `unit_price`, and threw while parsing.
    let line = &body["lines"][0];
    assert_eq!(line["sku"], "sku-bra-01", "line: {line}");
    assert_eq!(line["quantity"], 2, "line: {line}");
    assert!(line["unitPrice"]["amountMinor"].is_i64(), "line.unitPrice: {line}");
    assert!(line["unitPrice"]["currency"].is_string(), "line.unitPrice: {line}");
    assert!(line.get("unit_amount_minor").is_none(), "line: {line}");

    // `Money` nested, camelCase — the total of two 8900-yen bras.
    assert_eq!(body["total"]["amountMinor"], 17800, "total: {body}");
    assert_eq!(body["total"]["currency"], "JPY", "total: {body}");

    // `TrailEntry`: sequence, event, from, to — `from` and `to`, not
    // `from_state` and `to_state`.
    let trail = body["timeline"].as_array().expect("timeline is an array");
    assert!(!trail.is_empty(), "the saga recorded a trail: {body}");
    assert!(trail[0]["sequence"].is_i64(), "trail: {trail:?}");
    assert!(trail[0]["event"].is_string(), "trail: {trail:?}");
    assert!(trail[0]["from"].is_string(), "trail.from: {trail:?}");
    assert!(trail[0]["to"].is_string(), "trail.to: {trail:?}");
    assert!(trail[0].get("from_state").is_none(), "trail: {trail:?}");
    assert!(trail[0].get("to_state").is_none(), "trail: {trail:?}");

    // The client's price claim was discarded: the catalogue price won.
    assert_eq!(
        line["unitPrice"]["amountMinor"], 8900,
        "the server must price from the catalogue, not from the client's claim"
    );
}

#[tokio::test]
async fn a_checkout_that_sends_the_contract_line_is_accepted() {
    // The contract's `OrderLineInput` requires `sku` and `quantity`. The
    // price is optional and ignored for billing — the server resolves it
    // from the catalogue. The service used to require the price anyway, so
    // a client that sent the contract's line as written was rejected with
    // 422 before the order was ever priced.
    let gateway = gateway();
    let (status, body) = call(
        &gateway,
        "POST",
        "/v1/checkouts",
        Some(json!({
            "lines": [{"sku": "sku-bra-01", "quantity": 1}],
            "paymentMethodRef": "tok_test",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");

    // The response carries the order id the client navigates to.
    assert!(body["orderId"].is_string(), "orderId: {body}");
    assert!(body["terminal"].is_boolean(), "terminal: {body}");
    assert!(
        body["awaitingReconciliation"].is_boolean(),
        "awaitingReconciliation: {body}"
    );
}
