//! Content service — the editable content behind the DOKIPON storefront.
//!
//! The operator edits content here; the storefront reads it. Everything is
//! one document, versioned, and written through a single API so the admin
//! console has a coherent model of what it is editing.
//!
//! ## Why a service and not a config file
//!
//! A config file means a deploy to change a banner, a deploy to add a
//! collection, and two operators editing the same YAML. The operator needs
//! to change the shop between a Tuesday and a Friday without a release.
//!
//! ## Storage
//!
//! The store is in memory, behind a `RwLock`, with the document written
//! through to a JSON file so a restart does not lose the operator's work.
//! That is deliberately not a database: the content is one document, the
//! whole thing fits in memory, and the write path is a single atomic file
//! replace. Swapping in Postgres later means changing this one module.

use std::{
    fs,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use commerce_core::error::{status, AppError};
use commerce_core::JsonBody;
use serde::{Deserialize, Serialize};

mod model;

pub use model::{
    Banner, Collection, Faq, InfoPage, SectionFill, Series, SizeGuide, SizeOption, SizeRow,
    StorefrontContent, Topic, Trend,
};

pub type SharedContent = Arc<RwLock<ContentStore>>;

/// The content document plus the metadata the admin console needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentDocument {
    #[serde(flatten)]
    pub content: StorefrontContent,
    /// Bumped on every write. The admin console sends it back on a write and
    /// the service refuses a write against a stale version, so two operators
    /// cannot silently overwrite each other.
    pub revision: u64,
}

#[derive(Debug, Default)]
pub struct ContentStore {
    content: StorefrontContent,
    revision: u64,
    /// Where the document is persisted. None means "memory only".
    path: Option<PathBuf>,
}

impl ContentStore {
    pub fn new(path: Option<PathBuf>) -> Self {
        // Read the persisted document before the store takes ownership of the
        // path — the store owns it for the rest of its life.
        let (content, revision) = match &path {
            Some(path) => match fs::read(path) {
                Ok(bytes) => match serde_json::from_slice::<ContentDocument>(&bytes) {
                    Ok(doc) => (doc.content, doc.revision),
                    Err(e) => {
                        tracing::warn!(error = %e, "persisted content is unreadable; starting empty");
                        (StorefrontContent::default(), 0)
                    }
                },
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                    tracing::warn!(error = %e, "persisted content could not be read; starting empty");
                    (StorefrontContent::default(), 0)
                }
                Err(_) => (StorefrontContent::default(), 0),
            },
            None => (StorefrontContent::default(), 0),
        };
        Self {
            content,
            revision,
            path,
        }
    }

    pub fn document(&self) -> ContentDocument {
        ContentDocument {
            content: self.content.clone(),
            revision: self.revision,
        }
    }

    /// Replace the whole document.
    ///
    /// [expected_revision] is the revision the editor was looking at. A
    /// mismatch means someone else wrote in between, and the write is
    /// refused rather than merged — a content merge is an editorial decision,
    /// not something a service should guess at.
    pub fn replace(
        &mut self,
        content: StorefrontContent,
        expected_revision: u64,
    ) -> Result<ContentDocument, AppError> {
        const TRACE: &str = "content.replace";
        if expected_revision != self.revision {
            return Err(status::conflict(
                "content.revision_conflict",
                format!(
                    "the content was changed by someone else (you had revision {expected_revision}, it is now {})",
                    self.revision
                ),
                TRACE,
            ));
        }
        self.content = content;
        self.revision += 1;
        self.content.updated_at_ms = commerce_core::idempotency::now_ms();
        self.persist();
        Ok(self.document())
    }

    /// Write the document to disk, atomically.
    ///
    /// The write goes to a temporary file and is then renamed, so a crash
    /// mid-write cannot leave a half-written document where a whole one was.
    fn persist(&self) {
        let Some(path) = &self.path else { return };
        let doc = self.document();
        let Ok(json) = serde_json::to_vec_pretty(&doc) else {
            return;
        };
        let tmp = path.with_extension("json.tmp");
        if fs::write(&tmp, &json).is_ok() {
            let _ = fs::rename(&tmp, path);
        }
    }
}

/// A request to replace the whole document.
#[derive(Debug, Deserialize)]
struct ReplaceRequest {
    #[serde(flatten)]
    content: StorefrontContent,
    revision: u64,
}

pub fn router(state: SharedContent) -> Router {
    Router::new()
        .route("/v1/content", get(read_content))
        .route("/v1/content", put(replace_content))
        .route("/healthz", get(health))
        .with_state(state)
}

async fn health() -> StatusCode {
    StatusCode::OK
}

/// `GET /v1/content` — the whole document.
async fn read_content(
    State(store): State<SharedContent>,
) -> Result<Json<ContentDocument>, AppError> {
    let guard = store
        .read()
        .map_err(|_| AppError::Internal("content store lock poisoned".into()))?;
    Ok(Json(guard.document()))
}

/// `PUT /v1/content` — replace the whole document.
async fn replace_content(
    State(store): State<SharedContent>,
    JsonBody(request): JsonBody<ReplaceRequest>,
) -> Result<Json<ContentDocument>, AppError> {
    let mut guard = store
        .write()
        .map_err(|_| AppError::Internal("content store lock poisoned".into()))?;
    let doc = guard.replace(request.content, request.revision)?;
    Ok(Json(doc))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content_with_one_collection() -> StorefrontContent {
        StorefrontContent {
            collections: vec![Collection {
                id: "lingerie".to_string(),
                name: "lingerie".to_string(),
                title: "LINGERIE".to_string(),
                title_ja: "ランジェリー".to_string(),
                description: String::new(),
                position: 1,
                published: true,
                fill: SectionFill::Manual,
                product_ids: vec!["p-bra-01".to_string()],
                image_url: None,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn a_write_bumps_the_revision() {
        let mut store = ContentStore::new(None);
        assert_eq!(store.revision, 0);
        let doc = store
            .replace(content_with_one_collection(), 0)
            .expect("write");
        assert_eq!(doc.revision, 1);
        assert_eq!(doc.content.collections.len(), 1);
    }

    #[test]
    fn a_stale_revision_is_refused() {
        // Two operators opened the editor. The first one saved. The second
        // one's save must not silently overwrite the first one's work.
        let mut store = ContentStore::new(None);
        store
            .replace(content_with_one_collection(), 0)
            .expect("first write");
        let err = store
            .replace(StorefrontContent::default(), 0)
            .expect_err("stale write must be refused");
        match err {
            AppError::Problem(p) => {
                assert_eq!(p.status, 409);
                assert_eq!(p.code, "content.revision_conflict");
            }
            other => panic!("expected a problem, got {other:?}"),
        }
    }

    #[test]
    fn a_write_against_the_current_revision_succeeds() {
        let mut store = ContentStore::new(None);
        store
            .replace(content_with_one_collection(), 0)
            .expect("first write");
        let doc = store
            .replace(StorefrontContent::default(), 1)
            .expect("second write");
        assert_eq!(doc.revision, 2);
    }

    #[test]
    fn content_survives_a_restart() {
        // The operator's work must not vanish when the process restarts.
        let dir = std::env::temp_dir().join(format!("dokipon-content-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("content.json");

        {
            let mut store = ContentStore::new(Some(path.clone()));
            store
                .replace(content_with_one_collection(), 0)
                .expect("write");
        }

        let reloaded = ContentStore::new(Some(path.clone()));
        assert_eq!(reloaded.revision, 1);
        assert_eq!(reloaded.content.collections.len(), 1);
        assert_eq!(reloaded.content.collections[0].id, "lingerie");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unpublished_collection_is_not_served() {
        let mut store = ContentStore::new(None);
        let mut content = content_with_one_collection();
        content.collections[0].published = false;
        store.content = content;
        assert!(store.document().content.published_collections().is_empty());
    }
}
