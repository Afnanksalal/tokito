//! Schematic document round-trip (embedded Postgres via pg-embed).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn json_body(res: axum::response::Response) -> serde_json::Value {
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn put_and_get_schematic_document() {
    if !tokito::test_support::database_integration_tests_enabled() {
        return;
    }
    tokito::test_support::with_timeout(async {
        let (app, bearer) = tokito::test_support::test_router().await?;

        let create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/designs")
                    .header("authorization", bearer.as_str())
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"Doc test"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let design_id = json_body(create).await["id"].as_str().unwrap().to_string();

        let doc = tokito::models::SchematicDocument::empty();

        let put = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/v1/designs/{design_id}/schematic/document"))
                    .header("authorization", bearer.as_str())
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&doc).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(put.status(), StatusCode::OK);

        let get = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/designs/{design_id}/schematic/document"))
                    .header("authorization", bearer.as_str())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get.status(), StatusCode::OK);
        let loaded = json_body(get).await;
        assert_eq!(
            loaded["schema_version"],
            serde_json::json!(tokito::models::SCHEMATIC_DOCUMENT_SCHEMA_VERSION)
        );
        Ok(())
    })
    .await
    .expect("put_and_get_schematic_document");
}
