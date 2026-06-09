//! query-daemon — stateless axum server wrapping `query-core`. Generic over the
//! `Embedder`/`Searcher` impls so it can be tested with stubs and run with real
//! adapters. HTTP API is 1:1 with the MCP tools (ADR-0005).

use std::sync::Arc;

use axum::extract::Request;
use axum::extract::{Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;

use librarian_domain::{Embedder, Searcher, SourceId};
use query_core::{
    retrieval_confidence, ConfidenceLabel, QueryError, QueryService, RetrievalConfidence,
};

use crate::auth::{AuthReject, AuthState};

pub mod auth;
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

pub fn router<E, S>(state: AppState<E, S>, auth: Arc<AuthState>) -> Router
where
    E: Embedder + Send + Sync + 'static,
    S: Searcher + Send + Sync + 'static,
{
    // /v1/* require a valid bearer key (issue 032); /healthz stays open for monitoring.
    let protected = Router::new()
        .route("/v1/collections", get(collections::<E, S>))
        .route("/v1/search", post(search::<E, S>))
        .route("/v1/documents", get(documents::<E, S>))
        .route("/v1/extract", post(extract::<E, S>))
        .layer(axum::middleware::from_fn_with_state(auth, auth_mw))
        .with_state(state);
    Router::new()
        .route("/healthz", get(healthz))
        .merge(protected)
}

/// Extract the bearer token from `Authorization: Bearer <token>`.
fn bearer(headers: &axum::http::HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
}

/// Auth + rate-limit middleware for `/v1/*` (issue 032). On success the resolved `Identity` is
/// stashed in request extensions for downstream logging (issue 033).
async fn auth_mw(State(auth): State<Arc<AuthState>>, mut req: Request, next: Next) -> Response {
    match auth.check(bearer(req.headers())) {
        Ok(id) => {
            req.extensions_mut().insert(id);
            next.run(req).await
        }
        Err(AuthReject::Unauthorized) => ApiError {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized",
            message: "missing or invalid bearer key".into(),
            retry_after: None,
        }
        .into_response(),
        Err(AuthReject::RateLimited(secs)) => ApiError {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "rate_limited",
            message: "rate limit exceeded".into(),
            retry_after: Some(secs),
        }
        .into_response(),
    }
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

/// Tier 0 retrieval-confidence (issue 028): a reference-free triage signal, NOT a precise
/// grade (dense-QPP caveat — see docs/research/rag-quality/FINDINGS.md). Raw signals are
/// exposed so callers aren't reliant on the composite.
#[derive(Serialize)]
struct ConfidenceJson {
    value: f32,
    label: &'static str,
    top_score: f32,
    margin: f32,
    score_spread: f32,
    fragment_rate: f32,
}

impl From<RetrievalConfidence> for ConfidenceJson {
    fn from(c: RetrievalConfidence) -> Self {
        let label = match c.label {
            ConfidenceLabel::Strong => "strong",
            ConfidenceLabel::Weak => "weak",
            ConfidenceLabel::LikelyNoAnswer => "likely_no_answer",
        };
        Self {
            value: c.value,
            label,
            top_score: c.top_score,
            margin: c.margin,
            score_spread: c.score_spread,
            fragment_rate: c.fragment_rate,
        }
    }
}

#[derive(Serialize)]
struct SearchResp {
    hits: Vec<HitJson>,
    confidence: ConfidenceJson,
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
    let confidence = ConfidenceJson::from(retrieval_confidence(&hits));
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
        confidence,
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
