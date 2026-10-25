//! Catalog service — products, categories, SKUs with versioned authoritative pricing.
//!
//! Atlas invariant: **the server price is authoritative; client-submitted subtotals
//! are never trusted** (`API_EVENT_DATA_CONTRACTS.md` threat model).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use commerce_core::{
    error::{status, AppError},
    Currency, Money,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

pub type SharedCatalog = Arc<CatalogStore>;

/// At or below this on-hand count a `Reserved` SKU is advertised as LowStock.
pub const LOW_STOCK_THRESHOLD: i64 = 10;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sku {
    pub sku_id: String,
    pub product_id: String,
    pub amount_minor: i64,
    pub currency: Currency,
    pub version: i64,
    pub stock_policy: StockPolicy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StockPolicy {
    /// Digital goods: unlimited.
    Unbounded,
    /// Physical goods: must be reserved before payment.
    Reserved,
    /// Made to order: no stock concept.
    MadeToOrder,
}

impl StockPolicy {
    /// Only `Reserved` SKUs participate in the inventory Saga step.
    pub fn needs_inventory(self) -> bool {
        matches!(self, StockPolicy::Reserved)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelEligibility {
    pub web: bool,
    pub app: bool,
    pub line_mini_app: bool,
}

impl Default for ChannelEligibility {
    fn default() -> Self {
        Self {
            web: true,
            app: true,
            line_mini_app: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub product_id: String,
    pub sku: String,
    pub title_i18n: HashMap<String, String>,
    pub description_i18n: HashMap<String, String>,
    pub category_id: String,
    pub published: bool,
    /// Which client surfaces may show this product. LINE has stricter policy
    /// than web/app (atlas LIFF-010: policy-excluded content must not ship to LINE).
    pub channel_eligibility: ChannelEligibility,
}

/// One item of `GET /v1/products`.
///
/// The wire shape is fixed by the OpenAPI contract
/// (`contracts/openapi.yaml` -> `ProductSummary`): `productId`, `sku`,
/// `title`, `price` (a nested Money object) and `availability`.
///
/// The field names are camelCase and the price is NESTED, not flattened.
/// The client (packages/domain_models/lib/src/product.dart) reads
/// `json['price']['amountMinor']` and falls back to a flat `json['amountMinor']`
/// — a flat `amount_minor` is read by neither, so the storefront rendered every
/// price as "0 JPY" while the catalogue looked otherwise complete. The detail
/// endpoint in this same file already emitted the nested `price` object; the
/// list endpoint did not, so the two disagreed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductListItem {
    pub product_id: String,
    pub sku: String,
    pub title: String,
    pub price: Price,
    pub availability: Availability,
}

/// The `Money` shape the contract specifies: `{amountMinor, currency}`.
///
/// `commerce_core::Money` is a domain type without serde derives, so the wire
/// shape is spelled out here rather than reused directly. The camelCase
/// rename matters: `amount_minor` on the wire is read by nobody.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    pub amount_minor: i64,
    pub currency: Currency,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    InStock,
    LowStock,
    OutOfStock,
    MadeToOrder,
}

#[derive(Debug, Clone, Serialize)]
pub struct CursorPage<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

/// In-memory store. The atlas forbids claiming a production schema exists; this
/// is an explicit local store behind the same shape a Postgres implementation
/// would satisfy.
#[derive(Debug, Default)]
pub struct CatalogStore {
    pub products: HashMap<String, Product>,
    pub skus: HashMap<String, Sku>,
    /// Stock level reported by the inventory service; absent means "not reported".
    pub stock: HashMap<String, i64>,
}

impl CatalogStore {
    pub fn seed_demo() -> Self {
        let mut products = HashMap::new();
        let mut skus = HashMap::new();
        let mut stock = HashMap::new();

        // The demo catalogue is lingerie, because that is what DOKIPON sells.
        // The old seed was a generic apparel set (tees, hoodies, caps), which
        // made the storefront look like a merchandise store rather than the
        // thing it is.
        //
        // `p-adult-01` stays policy-restricted (web/app only, never LINE):
        // the channel-eligibility rule needs a product that actually exercises
        // it, and the "restricted" flag is a real product property, not a
        // styling choice.
        //
        // The categories are the ones the storefront's category strip offers
        // (`packages/design_system/.../storefront_app.dart` -> `_CategoryNav`).
        // The strip is a filter, not decoration: tapping an entry re-queries
        // the catalogue with that `categoryId`. A demo catalogue where most
        // of the entries lead to an empty shelf would make the filter look
        // broken, so every category the strip offers has at least one
        // product here. The ids are the ones the strip sends.
        let demo: [(
            &str,
            &str,
            &str,
            &str,
            i64,
            Currency,
            StockPolicy,
            i64,
            ChannelEligibility,
        ); 10] = [
            (
                "p-bra-01",
                "sku-bra-01",
                "DOKIPON Triple ノ-wire ブラ / Triple Wireless Bra",
                "cat-bra-sets",
                8900,
                Currency::Jpy,
                StockPolicy::Reserved,
                42,
                ChannelEligibility {
                    web: true,
                    app: true,
                    line_mini_app: true,
                },
            ),
            (
                "p-brief-01",
                "sku-brief-01",
                "DOKIPON 百合 ショーツ / Lily Brief",
                "cat-lingerie",
                3900,
                Currency::Jpy,
                StockPolicy::Reserved,
                7,
                ChannelEligibility {
                    web: true,
                    app: true,
                    line_mini_app: true,
                },
            ),
            (
                "p-set-01",
                "sku-set-01",
                "DOKIPON 黒蝶 ブラ＆ショーツセット / Black Butterfly Set",
                "cat-bra-sets",
                12900,
                Currency::Jpy,
                StockPolicy::Reserved,
                0,
                ChannelEligibility {
                    web: true,
                    app: true,
                    line_mini_app: true,
                },
            ),
            (
                "p-night-01",
                "sku-night-01",
                "DOKIPON 月光 ナイトブラ / Moonlight Night Bra",
                "cat-nightwear",
                4900,
                Currency::Jpy,
                StockPolicy::Reserved,
                28,
                ChannelEligibility {
                    web: true,
                    app: true,
                    line_mini_app: true,
                },
            ),
            (
                "p-cami-01",
                "sku-cami-01",
                "DOKIPON 霞 キャミソール / Haze Camisole",
                "cat-lingerie",
                6900,
                Currency::Jpy,
                StockPolicy::MadeToOrder,
                -1,
                ChannelEligibility {
                    web: true,
                    app: true,
                    line_mini_app: true,
                },
            ),
            (
                "p-adult-01",
                "sku-adult-01",
                "DOKIPON 深紅 ストッキング / Crimson Stockings",
                "cat-lingerie",
                5800,
                Currency::Jpy,
                StockPolicy::Reserved,
                3,
                ChannelEligibility {
                    web: true,
                    app: false,
                    line_mini_app: false,
                },
            ),
            (
                "p-swim-01",
                "sku-swim-01",
                "DOKIPON 潮騒 スイムビーフ / Shiosai Swim Brief",
                "cat-swim",
                5900,
                Currency::Jpy,
                StockPolicy::Reserved,
                12,
                ChannelEligibility {
                    web: true,
                    app: true,
                    line_mini_app: true,
                },
            ),
            (
                "p-beauty-01",
                "sku-beauty-01",
                "DOKIPON 雪輪 ボディケアセット / Yukiwa Body Care Set",
                "cat-beauty",
                2400,
                Currency::Jpy,
                StockPolicy::Unbounded,
                -1,
                ChannelEligibility {
                    web: true,
                    app: true,
                    line_mini_app: true,
                },
            ),
            (
                "p-goods-01",
                "sku-goods-01",
                "DOKIPON 星屑 ヘアピン / Hoshikuzu Hair Pin",
                "cat-goods",
                1800,
                Currency::Jpy,
                StockPolicy::Unbounded,
                -1,
                ChannelEligibility {
                    web: true,
                    app: true,
                    line_mini_app: true,
                },
            ),
            (
                "p-sale-01",
                "sku-sale-01",
                "DOKIPON 薄暮 セール セット / Twilight Sale Set",
                "cat-sale",
                3900,
                Currency::Jpy,
                StockPolicy::Reserved,
                5,
                ChannelEligibility {
                    web: true,
                    app: true,
                    line_mini_app: true,
                },
            ),
        ];

        for (pid, skuid, title, category, amount, cur, policy, qty, channels) in demo {
            let (ja, en) = title
                .split_once(" / ")
                .map_or((title.to_string(), title.to_string()), |(j, e)| {
                    (j.to_string(), e.to_string())
                });
            products.insert(
                pid.to_string(),
                Product {
                    product_id: pid.to_string(),
                    sku: skuid.to_string(),
                    title_i18n: HashMap::from([("ja".into(), ja), ("en".into(), en)]),
                    description_i18n: HashMap::new(),
                    category_id: category.into(),
                    published: true,
                    channel_eligibility: channels,
                },
            );
            skus.insert(
                skuid.to_string(),
                Sku {
                    sku_id: skuid.to_string(),
                    product_id: pid.to_string(),
                    amount_minor: amount,
                    currency: cur,
                    version: 1,
                    stock_policy: policy,
                },
            );
            // Only a `Reserved` SKU has a stock level. `Unbounded` and
            // `MadeToOrder` take no part in the inventory step
            // (`StockPolicy::needs_inventory`), so the catalogue must not
            // report a level for them: a level the inventory service does
            // not have is a SKU the two services disagree about, and it
            // shows up in the admin console as a product with "on hand 0"
            // that the inventory service never knew. The quantity column
            // is -1 for those SKUs, the same convention `sku-cami-01` uses.
            if policy == StockPolicy::Reserved {
                stock.insert(skuid.to_string(), qty);
            }
        }
        Self {
            products,
            skus,
            stock,
        }
    }

    fn availability(&self, sku: &Sku) -> Availability {
        match sku.stock_policy {
            StockPolicy::MadeToOrder => Availability::MadeToOrder,
            StockPolicy::Unbounded => Availability::InStock,
            StockPolicy::Reserved => match self.stock.get(&sku.sku_id) {
                None => Availability::InStock,
                Some(n) if *n <= 0 => Availability::OutOfStock,
                Some(n) if *n <= LOW_STOCK_THRESHOLD => Availability::LowStock,
                Some(_) => Availability::InStock,
            },
        }
    }

    /// Defensive fallback for unknown locales — the atlas requires a fallback
    /// rather than a blank or crashing.
    pub fn localize(map: &HashMap<String, String>, locale: &str) -> String {
        map.get(locale)
            .or_else(|| map.get("ja"))
            .or_else(|| map.values().next())
            .cloned()
            .unwrap_or_default()
    }
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
    pub locale: Option<String>,
    /// The category filter.
    ///
    /// The contract names the query parameter `categoryId`
    /// (`contracts/openapi.yaml` -> `GET /v1/products`) and the storefront
    /// sends exactly that. A query extractor matches field names literally,
    /// so without the rename `?categoryId=cat-swim` was dropped on the floor
    /// and the catalogue answered with the whole catalogue — a filter that
    /// looks wired up and filters nothing. `category_id` stays accepted as
    /// an alias for callers that spell it the Rust way.
    #[serde(rename = "categoryId", alias = "category_id")]
    pub category_id: Option<String>,
    /// `web` | `app` | `line` — BFF passes the end-client channel here.
    pub channel: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DetailPath {
    pub product_id: String,
}

pub fn router(state: SharedCatalog) -> Router {
    Router::new()
        .route("/v1/products", get(list_products))
        .route("/v1/products/{product_id}", get(get_product))
        .route("/healthz", get(health))
        .with_state(state)
}

async fn health() -> StatusCode {
    StatusCode::OK
}

/// `catalog.list` — cursor-paginated, locale-resolved, channel-filtered.
async fn list_products(
    State(store): State<SharedCatalog>,
    Query(q): Query<ListQuery>,
) -> Result<Json<CursorPage<ProductListItem>>, AppError> {
    let limit = q.limit.unwrap_or(20).clamp(1, 100) as usize;
    let locale = q.locale.as_deref().unwrap_or("ja");
    let offset = q
        .cursor
        .as_deref()
        .and_then(|c| c.parse::<usize>().ok())
        .unwrap_or(0);

    let mut sorted: Vec<(&String, &Product)> = store.products.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    let eligible: Vec<_> = sorted
        .iter()
        .filter(|(_, p)| p.published)
        .filter(|(_, p)| q.category_id.as_ref().is_none_or(|c| &p.category_id == c))
        .filter(|(_, p)| match q.channel.as_deref() {
            Some("line") => p.channel_eligibility.line_mini_app,
            Some("app") => p.channel_eligibility.app,
            _ => p.channel_eligibility.web,
        })
        .skip(offset)
        .take(limit + 1)
        .collect();

    let has_more = eligible.len() > limit;
    let page: Vec<ProductListItem> = eligible
        .into_iter()
        .take(limit)
        .filter_map(|(_, p)| {
            let sku = store.skus.get(&p.sku)?;
            Some(ProductListItem {
                product_id: p.product_id.clone(),
                sku: sku.sku_id.clone(),
                title: CatalogStore::localize(&p.title_i18n, locale),
                price: Price {
                    amount_minor: sku.amount_minor,
                    currency: sku.currency,
                },
                availability: store.availability(sku),
            })
        })
        .collect();

    let next_cursor = has_more.then(|| (offset + limit).to_string());

    Ok(Json(CursorPage {
        items: page,
        next_cursor,
    }))
}

/// `catalog.detail` — authoritative price + version, used by cart and checkout.
async fn get_product(
    State(store): State<SharedCatalog>,
    Path(p): Path<DetailPath>,
) -> Result<Json<serde_json::Value>, AppError> {
    let trace = "catalog.detail";
    let product = store.products.get(&p.product_id).ok_or_else(|| {
        status::not_found("catalog.product_not_found", "product not found", trace)
    })?;
    let sku = store.skus.get(&product.sku).ok_or_else(|| {
        AppError::Internal(format!(
            "catalog invariant violated: sku {} missing",
            product.sku
        ))
    })?;

    let price = Money::new(sku.amount_minor, sku.currency)
        .map_err(|e| AppError::Internal(format!("stored price invalid: {e}")))?;

    Ok(Json(serde_json::json!({
        "productId": product.product_id,
        "sku": sku.sku_id,
        "titleI18n": product.title_i18n,
        "descriptionI18n": product.description_i18n,
        "categoryId": product.category_id,
        "published": product.published,
        "channelEligibility": product.channel_eligibility,
        "price": { "amountMinor": price.amount_minor, "currency": price.currency },
        "version": sku.version,
        "stockPolicy": sku.stock_policy,
        "availability": store.availability(sku),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[test]
    fn seeded_products_have_positive_prices_and_versions() {
        let s = CatalogStore::seed_demo();
        assert!(!s.products.is_empty());
        for sku in s.skus.values() {
            assert!(sku.amount_minor > 0);
            assert!(sku.version >= 1);
        }
    }

    /// The list endpoint must speak the contract's wire shape, not the
    /// struct's field names.
    ///
    /// This is a regression guard: the list endpoint used to emit a flat
    /// `amount_minor` and `product_id`, while the contract (and the Flutter
    /// client) require `price.amountMinor` and `productId`. The storefront
    /// then rendered every price as "0 JPY" — a catalogue that looked
    /// complete but was priced at zero, and no test noticed, because the
    /// response was still 200 and still contained the right number of items.
    #[test]
    fn product_list_item_serializes_to_the_contract_shape() {
        let item = ProductListItem {
            product_id: "p-bra-01".to_string(),
            sku: "sku-bra-01".to_string(),
            title: "DOKIPON Triple ノ-wire ブラ".to_string(),
            price: Price {
                amount_minor: 4800,
                currency: Currency::Jpy,
            },
            availability: Availability::InStock,
        };

        let json = serde_json::to_value(&item).expect("serialize");

        assert_eq!(json["productId"], "p-bra-01");
        assert_eq!(json["sku"], "sku-bra-01");
        assert_eq!(json["title"], "DOKIPON Triple ノ-wire ブラ");
        // The price is nested under `price`, and its amount is `amountMinor`.
        assert_eq!(json["price"]["amountMinor"], 4800);
        assert_eq!(json["price"]["currency"], "JPY");
        assert_eq!(json["availability"], "in_stock");

        // The old flat shape must be gone: a client that prefers a flat
        // `amountMinor` over the nested object would read null.
        assert!(json.get("amount_minor").is_none());
        assert!(json.get("product_id").is_none());
        assert!(json.get("amountMinor").is_none());
    }

    #[test]
    fn localization_falls_back_to_ja_then_any() {
        let m = HashMap::from([("ja".to_string(), "JP".to_string())]);
        assert_eq!(CatalogStore::localize(&m, "en"), "JP");
        assert_eq!(CatalogStore::localize(&m, "ja"), "JP");
        assert_eq!(CatalogStore::localize(&HashMap::new(), "ja"), "");
    }

    #[test]
    fn only_reserved_policy_needs_inventory() {
        assert!(StockPolicy::Reserved.needs_inventory());
        assert!(!StockPolicy::Unbounded.needs_inventory());
        assert!(!StockPolicy::MadeToOrder.needs_inventory());
    }

    #[test]
    fn availability_reflects_stock() {
        let s = CatalogStore::seed_demo();
        assert_eq!(
            s.availability(s.skus.get("sku-bra-01").unwrap()),
            Availability::InStock
        );
        assert_eq!(
            s.availability(s.skus.get("sku-brief-01").unwrap()),
            Availability::LowStock
        );
        assert_eq!(
            s.availability(s.skus.get("sku-set-01").unwrap()),
            Availability::OutOfStock
        );
        assert_eq!(
            s.availability(s.skus.get("sku-cami-01").unwrap()),
            Availability::MadeToOrder
        );
        assert_eq!(
            s.availability(s.skus.get("sku-night-01").unwrap()),
            Availability::InStock
        );
    }

    #[test]
    fn policy_restricted_product_is_excluded_from_line_and_app() {
        let s = CatalogStore::seed_demo();
        let restricted = &s.products["p-adult-01"];
        assert!(!restricted.channel_eligibility.line_mini_app);
        assert!(!restricted.channel_eligibility.app);
        assert!(restricted.channel_eligibility.web);
        assert!(s.products["p-bra-01"].channel_eligibility.line_mini_app);
    }

    /// The storefront's category strip offers exactly these categories
    /// (`packages/design_system/.../storefront_app.dart` -> `_CategoryNav`),
    /// and tapping one of them filters the catalogue by that id. Every one of
    /// them must have at least one product in the demo catalogue: a strip
    /// whose entries mostly lead to an empty shelf makes a working filter
    /// look broken. This test is the seam between the Dart side and this
    /// side — if either renames a category, this fails.
    #[test]
    fn the_demo_catalogue_covers_every_category_the_strip_offers() {
        const STRIP: [&str; 7] = [
            "cat-lingerie",
            "cat-bra-sets",
            "cat-nightwear",
            "cat-swim",
            "cat-beauty",
            "cat-goods",
            "cat-sale",
        ];
        let s = CatalogStore::seed_demo();
        for category in STRIP {
            let n = s
                .products
                .values()
                .filter(|p| p.category_id == category)
                .count();
            assert!(
                n > 0,
                "category `{category}` has no product in the demo catalogue"
            );
        }
    }

    /// The category filter must work through the real query string.
    ///
    /// The contract names the parameter `categoryId`
    /// (`contracts/openapi.yaml` -> `GET /v1/products`) and the storefront
    /// sends exactly that. The filter used to be wired only to the Rust
    /// field name (`category_id`), and a query extractor matches field names
    /// literally — so `?categoryId=cat-swim` was dropped and the catalogue
    /// answered with every product. On screen that reads as a button that
    /// still does nothing. This test goes through the router, so it fails if
    /// the parameter name ever drifts from the contract again.
    #[tokio::test]
    async fn the_contract_category_parameter_filters_the_catalogue() {
        let app = router(Arc::new(CatalogStore::seed_demo()));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/products?categoryId=cat-swim")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let page: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let items = page["items"].as_array().expect("items is an array");

        assert_eq!(items.len(), 1, "the filter did not narrow the page: {page}");
        assert_eq!(items[0]["productId"], "p-swim-01");
    }

    /// A category with nothing in it is an empty page, not an error.
    ///
    /// The storefront renders a "no products in this category" state for it,
    /// and that state is only reachable if the server answers 200 with an
    /// empty list.
    #[tokio::test]
    async fn an_empty_category_is_an_empty_page_not_an_error() {
        let app = router(Arc::new(CatalogStore::seed_demo()));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/products?categoryId=cat-nothing-here")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let page: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(page["items"].as_array().map(Vec::len), Some(0));
        assert!(page["next_cursor"].is_null());
    }

    /// Without a category the whole catalogue comes back — the filter is an
    /// addition, not a replacement for the unfiltered listing.
    #[tokio::test]
    async fn an_unfiltered_listing_returns_the_whole_catalogue() {
        let app = router(Arc::new(CatalogStore::seed_demo()));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/products")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let page: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(page["items"].as_array().map(Vec::len), Some(10));
    }
}
