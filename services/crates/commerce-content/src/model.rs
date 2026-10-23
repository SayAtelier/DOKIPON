//! Content model for the DOKIPON storefront.
//!
//! The reference storefront (feliture.shop) is not just a product grid. It is
//! a content site wrapped around one: a category tree, a *series* tree (the
//! same product in several named lines), trend tags, editorial topics, a
//! ranking, size guidance, a shopping guide, an FAQ and a set of banners.
//!
//! All of that is **content**, not code. It changes with the season, with a
//! campaign, with a stockist's decision — and the operator must be able to
//! change it without a deploy. So it lives behind an API that the admin
//! console writes and the storefront reads.
//!
//! ## Design rules
//!
//! - **Content references products by id, never by copy.** A collection holds
//!   `product_id`s; the titles, prices and images come from the catalogue.
//!   Otherwise every campaign page would go stale the moment a price
//!   changed, and the same product would be described twice, differently.
//! - **Every content object is a separate document.** A campaign banner, a
//!   category and a FAQ entry have different lifetimes and different
//!   editors. Storing them in one blob means a typo in a FAQ breaks the
//!   banner.
//! - **Ordering is explicit.** A shop's homepage is an editorial argument:
//!   this set first, then that one. The order is data, not an accident of
//!   insertion order.

use serde::{Deserialize, Serialize};

/// How a section of the storefront is filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionFill {
    /// The section lists the products of one collection, in the collection's
    /// own order.
    Collection,
    /// The section lists products the operator picked, in the order they
    /// picked them.
    Manual,
    /// The section shows the catalogue's own "newest first" list.
    NewArrivals,
    /// The section shows the catalogue's own ranking.
    Ranking,
    /// The section shows the catalogue's best sellers.
    BestSellers,
}

/// A curated group of products — a collection, a campaign, a seasonal set.
///
/// This is the spine of the storefront. The reference site's `/collections/*`
/// pages and its "SET" / "LINGERIE" / "NEW ITEMS" home sections are all
/// collections; only the *source* of their membership differs, which is
/// what [SectionFill] records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    /// The operator-facing name, shown in the admin and used as a fallback
    /// label. The `title` is what the shopper sees.
    pub name: String,
    /// The shopper-facing heading, e.g. "LINGERIE".
    pub title: String,
    /// The shopper-facing sub-heading, e.g. "ランジェリー".
    pub title_ja: String,
    /// A short line under the heading.
    pub description: String,
    /// Where the section sits on the storefront. Lower numbers come first.
    pub position: i32,
    /// Whether the section is shown at all.
    pub published: bool,
    /// How the section's products are chosen.
    pub fill: SectionFill,
    /// For [SectionFill::Manual], the products, in order. Ignored otherwise.
    pub product_ids: Vec<String>,
    /// An optional image URL for the section's banner.
    pub image_url: Option<String>,
}

/// A named product line — "花束スフレブラ" is a series, and the same bra
/// appears under it and under "LINGERIE".
///
/// Series and collections are different things: a collection is a shelf, a
/// series is a design language. A product can be in many collections and
/// many series at once.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Series {
    pub id: String,
    pub name: String,
    pub title: String,
    pub title_ja: String,
    pub description: String,
    pub position: i32,
    pub published: bool,
    pub product_ids: Vec<String>,
}

/// A trend tag — "#スフレパッド", "#お泊まり", "#えろ可愛い".
///
/// These are how a lingerie shop talks about a product without a size chart:
/// a tag says what the product *is for*.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trend {
    pub id: String,
    /// The tag as shown, including the `#`.
    pub label: String,
    pub title: String,
    pub title_ja: String,
    pub description: String,
    pub position: i32,
    pub published: bool,
    pub product_ids: Vec<String>,
}

/// An editorial topic — the reference site's "TOPICS" band: a news item, a
/// campaign, a seasonal announcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub id: String,
    pub title: String,
    pub title_ja: String,
    pub summary: String,
    pub body: String,
    /// A link target: an internal route or an external URL.
    pub link_url: String,
    pub image_url: Option<String>,
    pub position: i32,
    pub published: bool,
    pub published_at_ms: i64,
}

/// A long-form information page — the shopping guide, the size guide, the
/// privacy policy, the terms.
///
/// The reference site has five of these and they are the pages a shopper
/// actually reads before buying a bra they have never bought before.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoPage {
    pub id: String,
    /// The slug used in the storefront's route, e.g. "shopping-guide".
    pub slug: String,
    pub title: String,
    pub title_ja: String,
    /// The page body, in a simple subset of Markdown.
    pub body: String,
    /// Where it appears in the footer, lower first.
    pub position: i32,
    pub published: bool,
}

/// A question and its answer — the reference site's FAQ block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Faq {
    pub id: String,
    pub question: String,
    pub question_ja: String,
    pub answer: String,
    pub answer_ja: String,
    /// The group the question belongs to, e.g. "ご注文について".
    pub category: String,
    pub position: i32,
    pub published: bool,
}

/// A promotional band on the storefront. The reference site's announcement
/// strip ("無料 shipping over ¥10,000", "最短当日発送") is made of these.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Banner {
    pub id: String,
    pub title: String,
    pub title_ja: String,
    pub body: String,
    pub link_url: Option<String>,
    /// An icon name the storefront maps to a glyph.
    pub icon: Option<String>,
    pub position: i32,
    pub published: bool,
}

/// A size the shopper can pick, and the measurements that go with it.
///
/// A lingerie shop without a size guide is a returns problem, not a missing
/// feature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeOption {
    /// The size label, e.g. "F" or "70B".
    pub code: String,
    /// The Japanese label, e.g. " testersFカップ".
    pub label_ja: String,
    /// Under-bust measurement in centimetres, when the product defines one.
    pub under_bust_cm: Option<i32>,
    /// Over-bust measurement in centimetres.
    pub over_bust_cm: Option<i32>,
    /// Waist measurement in centimetres.
    pub waist_cm: Option<i32>,
    /// Hip measurement in centimetres.
    pub hip_cm: Option<i32>,
    /// Whether the size is orderable right now.
    pub available: bool,
}

/// The size chart for one kind of garment, e.g. "ブラ" (bra) or
/// "ショーツ" (briefs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeGuide {
    /// The garment this chart applies to, e.g. "bra".
    pub id: String,
    pub name: String,
    pub name_ja: String,
    /// The measurement rows, in the order they should be shown.
    pub rows: Vec<SizeRow>,
    pub published: bool,
}

/// One measurement column of a [SizeGuide].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeRow {
    pub size: SizeOption,
    /// The extra per-size notes, e.g. "カップ: F".
    pub note: String,
}

/// Everything the storefront needs to render itself.
///
/// One endpoint, one document. The storefront fetches this at boot and
/// renders from it. Fetching each section separately would mean a page that
/// renders in pieces as seven requests land — and a shopper watching the
/// page assemble itself.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorefrontContent {
    pub collections: Vec<Collection>,
    pub series: Vec<Series>,
    pub trends: Vec<Trend>,
    pub topics: Vec<Topic>,
    pub pages: Vec<InfoPage>,
    pub faqs: Vec<Faq>,
    pub banners: Vec<Banner>,
    pub size_guides: Vec<SizeGuide>,
    /// When this document was last written, in epoch milliseconds.
    pub updated_at_ms: i64,
}

impl StorefrontContent {
    /// The collections that are published, in display order.
    pub fn published_collections(&self) -> Vec<&Collection> {
        let mut items: Vec<&Collection> = self.collections.iter().filter(|c| c.published).collect();
        items.sort_by_key(|c| c.position);
        items
    }

    /// The series that are published, in display order.
    pub fn published_series(&self) -> Vec<&Series> {
        let mut items: Vec<&Series> = self.series.iter().filter(|s| s.published).collect();
        items.sort_by_key(|s| s.position);
        items
    }

    /// The trends that are published, in display order.
    pub fn published_trends(&self) -> Vec<&Trend> {
        let mut items: Vec<&Trend> = self.trends.iter().filter(|t| t.published).collect();
        items.sort_by_key(|t| t.position);
        items
    }

    /// The topics that are published, newest first within their position.
    pub fn published_topics(&self) -> Vec<&Topic> {
        let mut items: Vec<&Topic> = self.topics.iter().filter(|t| t.published).collect();
        items.sort_by(|a, b| a.position.cmp(&b.position).then(b.published_at_ms.cmp(&a.published_at_ms)));
        items
    }

    /// The pages that are published, in display order.
    pub fn published_pages(&self) -> Vec<&InfoPage> {
        let mut items: Vec<&InfoPage> = self.pages.iter().filter(|p| p.published).collect();
        items.sort_by_key(|p| p.position);
        items
    }

    /// The FAQs that are published, in display order.
    pub fn published_faqs(&self) -> Vec<&Faq> {
        let mut items: Vec<&Faq> = self.faqs.iter().filter(|f| f.published).collect();
        items.sort_by_key(|f| f.position);
        items
    }

    /// The banners that are published, in display order.
    pub fn published_banners(&self) -> Vec<&Banner> {
        let mut items: Vec<&Banner> = self.banners.iter().filter(|b| b.published).collect();
        items.sort_by_key(|b| b.position);
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collection(id: &str, position: i32, published: bool) -> Collection {
        Collection {
            id: id.to_string(),
            name: id.to_string(),
            title: id.to_uppercase(),
            title_ja: id.to_string(),
            description: String::new(),
            position,
            published,
            fill: SectionFill::Manual,
            product_ids: vec![],
            image_url: None,
        }
    }

    #[test]
    fn unpublished_collections_are_hidden() {
        let content = StorefrontContent {
            collections: vec![
                collection("new", 1, true),
                collection("hidden", 2, false),
            ],
            ..Default::default()
        };
        let visible = content.published_collections();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, "new");
    }

    #[test]
    fn collections_are_ordered_by_position_not_insertion_order() {
        let content = StorefrontContent {
            collections: vec![
                collection("third", 30, true),
                collection("first", 10, true),
                collection("second", 20, true),
            ],
            ..Default::default()
        };
        let ids: Vec<&str> = content
            .published_collections()
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        assert_eq!(ids, vec!["first", "second", "third"]);
    }

    #[test]
    fn a_content_document_round_trips_through_json() {
        // The admin console writes this and the storefront reads it, so the
        // shape has to survive a round trip unchanged.
        let content = StorefrontContent {
            collections: vec![collection("lingerie", 1, true)],
            series: vec![Series {
                id: "hanataba".to_string(),
                name: "hanataba".to_string(),
                title: "花束スフレブラ".to_string(),
                title_ja: "花束スフレブラ".to_string(),
                description: "A bra shaped like a bouquet of cream puffs.".to_string(),
                position: 1,
                published: true,
                product_ids: vec!["p-bra-01".to_string()],
            }],
            trends: vec![Trend {
                id: "souffle".to_string(),
                label: "#スフレパッド".to_string(),
                title: "Soufflé pad".to_string(),
                title_ja: "スフレパッド".to_string(),
                description: String::new(),
                position: 1,
                published: true,
                product_ids: vec!["p-bra-01".to_string()],
            }],
            topics: vec![Topic {
                id: "t1".to_string(),
                title: "Anniversary".to_string(),
                title_ja: "周年セット".to_string(),
                summary: "A limited set.".to_string(),
                body: "Longer text.".to_string(),
                link_url: "/collections/anniversary".to_string(),
                image_url: None,
                position: 1,
                published: true,
                published_at_ms: 1_700_000_000_000,
            }],
            pages: vec![InfoPage {
                id: "p1".to_string(),
                slug: "shopping-guide".to_string(),
                title: "Shopping guide".to_string(),
                title_ja: "ショッピングガイド".to_string(),
                body: "# Guide".to_string(),
                position: 1,
                published: true,
            }],
            faqs: vec![Faq {
                id: "f1".to_string(),
                question: "Can I return?".to_string(),
                question_ja: "返品できますか？".to_string(),
                answer: "Defects only.".to_string(),
                answer_ja: "欠陥品のみ。".to_string(),
                category: "返品・交換について".to_string(),
                position: 1,
                published: true,
            }],
            banners: vec![Banner {
                id: "b1".to_string(),
                title: "Free shipping over ¥10,000".to_string(),
                title_ja: "10,000円以上で送料無料".to_string(),
                body: String::new(),
                link_url: Some("/pages/shopping-guide".to_string()),
                icon: Some("shipping".to_string()),
                position: 1,
                published: true,
            }],
            size_guides: vec![SizeGuide {
                id: "bra".to_string(),
                name: "Bra".to_string(),
                name_ja: "ブラ".to_string(),
                rows: vec![SizeRow {
                    size: SizeOption {
                        code: "F".to_string(),
                        label_ja: "Fカップ".to_string(),
                        under_bust_cm: Some(68),
                        over_bust_cm: Some(83),
                        waist_cm: None,
                        hip_cm: None,
                        available: true,
                    },
                    note: "カップ: F".to_string(),
                }],
                published: true,
            }],
            updated_at_ms: 1_700_000_000_000,
        };

        let json = serde_json::to_string(&content).expect("serialize");
        let back: StorefrontContent = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(back.collections.len(), 1);
        assert_eq!(back.collections[0].id, "lingerie");
        assert_eq!(back.series[0].product_ids, vec!["p-bra-01"]);
        assert_eq!(back.trends[0].label, "#スフレパッド");
        assert_eq!(back.topics[0].published_at_ms, 1_700_000_000_000);
        assert_eq!(back.pages[0].slug, "shopping-guide");
        assert_eq!(back.faqs[0].category, "返品・交換について");
        assert_eq!(back.banners[0].link_url.as_deref(), Some("/pages/shopping-guide"));
        assert_eq!(back.size_guides[0].rows[0].size.under_bust_cm, Some(68));
        assert_eq!(back.updated_at_ms, 1_700_000_000_000);
    }

    #[test]
    fn a_collection_references_products_by_id_not_by_copy() {
        // The contract the content layer must keep: a collection holds ids,
        // never titles or prices. This is the property that stops the
        // storefront from showing a stale price.
        let c = collection("lingerie", 1, true);
        let json = serde_json::to_value(&c).expect("serialize");
        assert!(json.get("product_ids").is_some());
        assert!(json.get("products").is_none());
        assert!(json.get("price").is_none());
    }
}
