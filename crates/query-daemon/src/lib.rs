//! query-daemon — stateless axum server wrapping `query-core`. Generic over the
//! `Embedder`/`Searcher` impls so it can be tested with stubs and run with real
//! adapters. HTTP API is 1:1 with the MCP tools (ADR-0005).

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;

use librarian_domain::{Embedder, Searcher, SourceId};
use query_core::{QueryError, QueryService};

pub mod config;

#[cfg(feature = "test-support")]
pub mod test_support;

pub struct AppState<E, S> {
    pub svc: Arc<QueryService<E, S>>,
}

impl<E, S> Clone for AppState<E, S> {
    fn clone(&self) -> Self {
        Self {
            svc: Arc::clone(&self.svc),
        }
    }
}

pub fn router<E, S>(state: AppState<E, S>) -> Router
where
    E: Embedder + Send + Sync + 'static,
    S: Searcher + Send + Sync + 'static,
{
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/collections", get(collections::<E, S>))
        .route("/v1/search", post(search::<E, S>))
        .route("/v1/documents", get(documents::<E, S>))
        .route("/v1/extract", post(extract::<E, S>))
        .with_state(state)
}

// ---- error -> HTTP (ADR-0005 table) -------------------------------------

struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    retry_after: Option<u64>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({"error": {"code": self.code, "message": self.message}}));
        let mut resp = (self.status, body).into_response();
        if let Some(secs) = self.retry_after {
            resp.headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from(secs));
        }
        resp
    }
}

impl From<QueryError> for ApiError {
    fn from(e: QueryError) -> Self {
        use librarian_domain::SearchError;
        let msg = e.to_string();
        match e {
            QueryError::EmptyQuery => ApiError {
                status: StatusCode::BAD_REQUEST,
                code: "bad_request",
                message: msg,
                retry_after: None,
            },
            QueryError::EmbedRecoverable(_) => ApiError {
                status: StatusCode::SERVICE_UNAVAILABLE,
                code: "embedder_unavailable",
                message: msg,
                retry_after: Some(2),
            },
            QueryError::EmbedTerminal(_) => ApiError {
                status: StatusCode::BAD_GATEWAY,
                code: "embedder_failed",
                message: msg,
                retry_after: None,
            },
            QueryError::EmbedPanic => ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "internal",
                message: msg,
                retry_after: None,
            },
            QueryError::Search(SearchError::NotFound(_)) => ApiError {
                status: StatusCode::NOT_FOUND,
                code: "not_found",
                message: msg,
                retry_after: None,
            },
            QueryError::Search(SearchError::Unavailable(_)) => ApiError {
                status: StatusCode::SERVICE_UNAVAILABLE,
                code: "search_unavailable",
                message: msg,
                retry_after: Some(2),
            },
            QueryError::Search(SearchError::Backend(_)) => ApiError {
                status: StatusCode::BAD_GATEWAY,
                code: "search_error",
                message: msg,
                retry_after: None,
            },
        }
    }
}

// ---- DTOs ----------------------------------------------------------------

#[derive(Deserialize)]
struct SearchReq {
    collection: String,
    query: String,
    #[serde(default)]
    limit: Option<u64>,
    #[serde(default)]
    content_type: Option<String>,
}

#[derive(Serialize)]
struct HitJson {
    score: f32,
    source_id: String,
    content_type: String,
    chunk_index: u32,
    text: String,
}

#[derive(Serialize)]
struct SearchResp {
    hits: Vec<HitJson>,
}

#[derive(Deserialize)]
struct CollectionQuery {
    collection: String,
}

#[derive(Serialize)]
struct DocumentsResp {
    documents: Vec<String>,
}

#[derive(Deserialize)]
struct ExtractReq {
    collection: String,
    source_id: String,
    #[serde(default)]
    start: Option<u32>,
    #[serde(default)]
    end: Option<u32>,
}

#[derive(Serialize)]
struct ExtractChunkJson {
    chunk_index: u32,
    text: String,
}

#[derive(Serialize)]
struct ExtractResp {
    source_id: String,
    chunks: Vec<ExtractChunkJson>,
}

#[derive(Serialize)]
struct CollectionJson {
    name: String,
    points: u64,
}

#[derive(Serialize)]
struct CollectionsResp {
    collections: Vec<CollectionJson>,
}

// ---- handlers ------------------------------------------------------------

async fn healthz() -> Json<serde_json::Value> {
    Json(json!({"status": "ok"}))
}

async fn search<E, S>(
    State(st): State<AppState<E, S>>,
    Json(req): Json<SearchReq>,
) -> Result<Json<SearchResp>, ApiError>
where
    E: Embedder + Send + Sync + 'static,
    S: Searcher + Send + Sync + 'static,
{
    let limit = req.limit.unwrap_or(5);
    let hits = st
        .svc
        .search(
            &req.collection,
            &req.query,
            limit,
            req.content_type.as_deref(),
        )
        .await?;
    Ok(Json(SearchResp {
        hits: hits
            .into_iter()
            .map(|h| HitJson {
                score: h.score,
                source_id: h.source_id.0,
                content_type: h.content_type,
                chunk_index: h.chunk_index,
                text: h.text,
            })
            .collect(),
    }))
}

async fn documents<E, S>(
    State(st): State<AppState<E, S>>,
    Query(q): Query<CollectionQuery>,
) -> Result<Json<DocumentsResp>, ApiError>
where
    E: Embedder + Send + Sync + 'static,
    S: Searcher + Send + Sync + 'static,
{
    let docs = st.svc.list_documents(&q.collection).await?;
    Ok(Json(DocumentsResp {
        documents: docs.into_iter().map(|s| s.0).collect(),
    }))
}

async fn extract<E, S>(
    State(st): State<AppState<E, S>>,
    Json(req): Json<ExtractReq>,
) -> Result<Json<ExtractResp>, ApiError>
where
    E: Embedder + Send + Sync + 'static,
    S: Searcher + Send + Sync + 'static,
{
    let sid = SourceId(req.source_id);
    let chunks = st
        .svc
        .get_extract(
            &req.collection,
            &sid,
            req.start.unwrap_or(0),
            req.end.unwrap_or(u32::MAX),
        )
        .await?;
    Ok(Json(ExtractResp {
        source_id: sid.0,
        chunks: chunks
            .into_iter()
            .map(|c| ExtractChunkJson {
                chunk_index: c.chunk_index,
                text: c.text,
            })
            .collect(),
    }))
}

async fn collections<E, S>(
    State(st): State<AppState<E, S>>,
) -> Result<Json<CollectionsResp>, ApiError>
where
    E: Embedder + Send + Sync + 'static,
    S: Searcher + Send + Sync + 'static,
{
    let cols = st.svc.list_collections().await?;
    Ok(Json(CollectionsResp {
        collections: cols
            .into_iter()
            .map(|c| CollectionJson {
                name: c.name,
                points: c.points,
            })
            .collect(),
    }))
}
