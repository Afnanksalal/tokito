//! API integration tests — run against PostgreSQL (`TOKITO_TEST_DATABASE_URL`).
//!
//! Local: `docker compose up -d` then
//! `set TOKITO_TEST_DATABASE_URL=postgres://tokito:tokito@localhost:5433/tokito_test`
//! `cargo test -p tokito --test integration -- --ignored --nocapture`

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::postgres::PgPoolOptions;
use tokito::router;
use tower::ServiceExt;

#[tokio::test]
#[ignore = "requires TOKITO_TEST_DATABASE_URL and Postgres"]
async fn health_returns_ok() {
    let url = std::env::var("TOKITO_TEST_DATABASE_URL")
        .expect("set TOKITO_TEST_DATABASE_URL for integration tests");
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&url)
        .await
        .expect("connect test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    let app = router::build(router::AppState::test(pool), vec![], None);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["status"], "ok");
}
