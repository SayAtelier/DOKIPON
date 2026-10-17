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
        Self { web: true, app: true, line_mini_app: true }
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

#[derive(Debug, Clone, Serialize)]
pub struct ProductListItem {
    pub product_id: String,
    pub sku: String,
    pub title: String,
    pub amount_minor: i64,
    pub currency: Currency,
    pub availability: Availability,
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

        let demo: [(&str, &str, &str, i64, Currency, StockPolicy, i64, ChannelEligibility); 6] = [
            ("p-tee-01", "sku-tee-01", "GemArc ロゴTシャツ / GemArc Logo Tee", 4800, Currency::Jpy, StockPolicy::Reserved, 42, ChannelEligibility { web: true, app: true, line_mini_app: true }),
            ("p-hoodie-01", "sku-hoodie-01", "GemArc パーカー / GemArc Hoodie", 12800, Currency::Jpy, StockPolicy::Reserved, 7, ChannelEligibility { web: true, app: true, line_mini_app: true }),
            ("p-cap-01", "sku-cap-01", "GemArc キャップ / GemArc Cap", 3900, Currency::Jpy, StockPolicy::Reserved, 0, ChannelEligibility { web: true, app: true, line_mini_app: true }),
            ("p-sticker", "sku-sticker", "GemArc ステッカーセット / Sticker Set", 800, Currency::Jpy, StockPolicy::Unbounded, -1, ChannelEligibility { web: true, app: true, line_mini_app: true }),
            ("p-print-01", "sku-print-01", "GemArc プリント（受注生産） / Made-to-order Print", 9800, Currency::Jpy, StockPolicy::MadeToOrder, -1, ChannelEligibility { web: true, app: true, line_mini_app: true }),
            ("p-adult-01", "sku-adult-01", "Policy-restricted sample / ポリシー要審査", 9800, Currency::Jpy, StockPolicy::Reserved, 3, ChannelEligibility { web: true, app: false, line_mini_app: false }),
        ];

        for (pid, skuid, title, amount, cur, policy, qty, channels) in demo {
            let (ja, en) = title.split_once(" / ").map_or((title.to_string(), title.to_string()), |(j, e)| (j.to_string(), e.to_string()));
            products.insert(
                pid.to_string(),
                Product {
                    product_id: pid.to_string(),
                    sku: skuid.to_string(),
                    title_i18n: HashMap::from([("ja".into(), ja), ("en".into(), en)]),
                    description_i18n: HashMap::new(),
                    category_id: "cat-apparel".into(),
                    published: true,
                    channel_eligibility: channels,
                },
            );
            skus.insert(
                skuid.to_string(),
                Sku { sku_id: skuid.to_string(), product_id: pid.to_string(), amount_minor: amount, currency: cur, version: 1, stock_policy: policy },
            );
            if qty >= 0 {
                stock.insert(skuid.to_string(), qty);
            }
        }
        Self { products, skus, stock }
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
                amount_minor: sku.amount_minor,
                currency: sku.currency,
                availability: store.availability(sku),
            })
        })
        .collect();

    let next_cursor = has_more.then(|| (offset + limit).to_string());

    Ok(Json(CursorPage { items: page, next_cursor }))
}

/// `catalog.detail` — authoritative price + version, used by cart and checkout.
async fn get_product(
    State(store): State<SharedCatalog>,
    Path(p): Path<DetailPath>,
) -> Result<Json<serde_json::Value>, AppError> {
    let trace = "catalog.detail";
    let product = store
        .products
        .get(&p.product_id)
        .ok_or_else(|| status::not_found("catalog.product_not_found", "product not found", trace))?;
    let sku = store
        .skus
        .get(&product.sku)
        .ok_or_else(|| AppError::Internal(format!("catalog invariant violated: sku {} missing", product.sku)))?;

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

/// Run this service standalone. In the composed topology the BFF calls it
/// over HTTP; this exists so `cargo run -p commerce-catalog` is usable.
#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let store: SharedCatalog = Arc::new(CatalogStore::seed_demo());
    let listener = tokio::net::TcpListener::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("bind catalog");
    tracing::info!("catalog service listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, router(store)).await.expect("serve catalog");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_products_have_positive_prices_and_versions() {
        let s = CatalogStore::seed_demo();
        assert!(!s.products.is_empty());
        for sku in s.skus.values() {
            assert!(sku.amount_minor > 0);
            assert!(sku.version >= 1);
        }
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
        assert_eq!(s.availability(s.skus.get("sku-tee-01").unwrap()), Availability::InStock);
        assert_eq!(s.availability(s.skus.get("sku-hoodie-01").unwrap()), Availability::LowStock);
        assert_eq!(s.availability(s.skus.get("sku-cap-01").unwrap()), Availability::OutOfStock);
        assert_eq!(s.availability(s.skus.get("sku-print-01").unwrap()), Availability::MadeToOrder);
        assert_eq!(s.availability(s.skus.get("sku-sticker").unwrap()), Availability::InStock);
    }

    #[test]
    fn policy_restricted_product_is_excluded_from_line_and_app() {
        let s = CatalogStore::seed_demo();
        let restricted = &s.products["p-adult-01"];
        assert!(!restricted.channel_eligibility.line_mini_app);
        assert!(!restricted.channel_eligibility.app);
        assert!(restricted.channel_eligibility.web);
        assert!(s.products["p-tee-01"].channel_eligibility.line_mini_app);
    }
}
