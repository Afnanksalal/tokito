//! Axum router, shared [`AppState`], and middleware (CORS, tracing).

use crate::config::{FirecrawlConfig, NexarConfig, XaiConfig};
use crate::handlers;
use axum::body::Body;
use axum::http::{header, StatusCode};
use axum::http::{HeaderValue, Method};
use axum::middleware::from_fn_with_state;
use axum::response::Response;
use axum::routing::{delete, get, post};
use axum::Router;
use sqlx::PgPool;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

/// Shared application state (database pool + HTTP client for integrations).
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub http: reqwest::Client,
    pub jwt_secret: String,
    pub xai: Option<XaiConfig>,
    pub firecrawl: Option<FirecrawlConfig>,
    pub nexar: Option<NexarConfig>,
    #[allow(clippy::type_complexity)]
    pub nexar_token_cache: Option<Arc<Mutex<Option<(String, std::time::Instant)>>>>,
    pub lcsc_anonymous_search: bool,
    pub agent: crate::config::AgentLimits,
}

impl AppState {
    /// Builds state for production using loaded [`crate::config::Config`].
    pub fn try_new(pool: PgPool, cfg: &crate::config::Config) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(15))
            .user_agent(concat!("tokito/", env!("CARGO_PKG_VERSION")))
            .build()?;
        let nexar_token_cache = cfg.nexar.as_ref().map(|_| Arc::new(Mutex::new(None)));
        Ok(Self {
            pool,
            http,
            jwt_secret: cfg.jwt_secret.clone(),
            xai: cfg.xai.clone(),
            firecrawl: cfg.firecrawl.clone(),
            nexar: cfg.nexar.clone(),
            nexar_token_cache,
            lcsc_anonymous_search: cfg.lcsc_anonymous_search,
            agent: cfg.agent.clone(),
        })
    }

    /// Minimal state for tests (no API keys; default HTTP client).
    pub fn test(pool: PgPool) -> Self {
        Self {
            pool,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
            jwt_secret: "unit-test-jwt-secret-do-not-use".to_string(),
            xai: None,
            firecrawl: None,
            nexar: None,
            nexar_token_cache: None,
            lcsc_anonymous_search: false,
            agent: crate::config::AgentLimits {
                max_iterations: 2,
                max_llm_tokens_per_run: 4096,
                default_model: "grok-4.3".to_string(),
            },
        }
    }
}

/// Builds the HTTP [`Router`] with `/health` and versioned API routes.
///
/// When `spa_static_dir` is set to a built frontend directory, unknown
/// `GET` routes fall through to that folder and then to `index.html` so client-side routes like
/// `/app` work on the same origin as the API.
pub fn build(
    state: AppState,
    cors_origins: Vec<String>,
    spa_static_dir: Option<PathBuf>,
) -> Router {
    let cors = build_cors_layer(cors_origins);
    let st = state.clone();

    let auth_routes = Router::new()
        .route("/register", post(handlers::register))
        .route("/login", post(handlers::login))
        .merge(
            Router::new()
                .route(
                    "/api-keys",
                    get(handlers::list_api_keys).post(handlers::create_api_key),
                )
                .route("/api-keys/:id", delete(handlers::delete_api_key))
                .route_layer(from_fn_with_state(
                    st.clone(),
                    crate::auth::middleware::require_auth,
                )),
        );

    let protected = Router::new()
        .route(
            "/integrations/xai/chat/completions",
            post(handlers::xai_chat_completions),
        )
        .route(
            "/integrations/firecrawl/scrape",
            post(handlers::firecrawl_scrape),
        )
        .route(
            "/integrations/firecrawl/search",
            post(handlers::firecrawl_search),
        )
        .route(
            "/manufacturers",
            get(handlers::list_mfg).post(handlers::create_mfg),
        )
        .route("/catalog/search", get(handlers::search_catalog))
        .route(
            "/parts",
            get(handlers::search_parts).post(handlers::create_part),
        )
        .route("/parts/:id", get(handlers::get_part))
        .route(
            "/parts/:id/offers",
            get(handlers::list_part_offers).post(handlers::sync_part_offers),
        )
        .route(
            "/designs",
            get(handlers::list_designs).post(handlers::create_design),
        )
        .route(
            "/designs/:id",
            get(handlers::get_design).patch(handlers::patch_design),
        )
        .route(
            "/designs/:id/intent",
            get(handlers::get_intent).put(handlers::put_intent),
        )
        .route("/designs/:id/research", get(handlers::list_research))
        .route(
            "/designs/:id/research/scrape",
            post(handlers::scrape_research),
        )
        .route(
            "/designs/:id/research/search",
            post(handlers::search_research),
        )
        .route("/designs/:id/export", get(handlers::export_design))
        .route(
            "/designs/:id/bom",
            get(handlers::get_bom).put(handlers::put_bom),
        )
        .route("/designs/:id/bom/append", post(handlers::append_bom))
        .route(
            "/designs/:id/schematic",
            get(handlers::get_schematic).put(handlers::put_schematic),
        )
        .route(
            "/designs/:id/schematic/document",
            get(handlers::get_schematic_document).put(handlers::put_schematic_document),
        )
        .route(
            "/designs/:id/schematic/suggest",
            post(handlers::suggest_schematic),
        )
        .route(
            "/designs/:id/schematic/validate",
            post(handlers::validate_schematic_payload),
        )
        .route("/agent/run", post(handlers::run_agent))
        .route_layer(from_fn_with_state(
            st,
            crate::auth::middleware::require_auth,
        ));

    let api = Router::new()
        .route("/health", get(handlers::health))
        .nest(
            "/v1",
            Router::new().nest("/auth", auth_routes).merge(protected),
        )
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    if let Some(dir) = spa_static_dir {
        let index = dir.join("index.html");
        if dir.is_dir() && index.is_file() {
            let index_file = ServeFile::new(index.clone());
            let index_ok = tower::service_fn(move |req: axum::http::Request<Body>| {
                let index = index.clone();
                let uri_path = req.uri().path().to_string();
                async move {
                    if uri_path.starts_with("/v1") || uri_path == "/health" {
                        return Ok::<Response, std::convert::Infallible>(
                            Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .body(Body::empty())
                                .unwrap(),
                        );
                    }

                    match tokio::fs::read(&index).await {
                        Ok(bytes) => Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                            .body(Body::from(bytes))
                            .unwrap()),
                        Err(_) => Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::from("failed to read index.html"))
                            .unwrap()),
                    }
                }
            });

            // Serve static assets when present; for SPA routes (e.g. /app) return index.html with 200.
            return api.fallback_service(
                ServeDir::new(dir)
                    .not_found_service(index_ok)
                    // still allow direct /index.html to be served normally
                    .fallback(index_file),
            );
        }
        tracing::warn!(
            ?dir,
            "TOKITO_STATIC_DIR is not a directory with index.html; SPA hosting disabled"
        );
    }

    api
}

fn build_cors_layer(origins: Vec<String>) -> CorsLayer {
    if origins.is_empty() {
        return CorsLayer::permissive();
    }
    let mut list = Vec::with_capacity(origins.len());
    for o in origins {
        if let Ok(h) = HeaderValue::from_str(&o) {
            list.push(h);
        }
    }
    if list.is_empty() {
        return CorsLayer::permissive();
    }
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(list))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(tower_http::cors::Any)
}
