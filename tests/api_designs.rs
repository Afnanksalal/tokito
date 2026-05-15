//! Design CRUD over HTTP (embedded Postgres via pg-embed).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn json_body(res: axum::response::Response) -> serde_json::Value {
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn create_and_list_designs() {
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
                    .body(Body::from(
                        r#"{"name":"Integration board","description":"test"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create.status(), StatusCode::OK);
        let design = json_body(create).await;
        let id = design["id"].as_str().expect("design id");

        let list = app
            .oneshot(
                Request::builder()
                    .uri("/v1/designs")
                    .header("authorization", bearer.as_str())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list.status(), StatusCode::OK);
        let rows = json_body(list).await;
        assert!(rows.as_array().unwrap().iter().any(|d| d["id"] == id));
        Ok(())
    })
    .await
    .expect("create_and_list_designs");
}
