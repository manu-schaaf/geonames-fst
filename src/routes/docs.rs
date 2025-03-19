use aide::swagger::Swagger;
use aide::{
    axum::routing::{get, get_with},
    axum::{ApiRouter, IntoApiResponse},
    openapi::OpenApi,
};
use axum::response::Redirect;
use axum::{response::IntoResponse, Extension, Json};

use crate::AppState;

pub(crate) fn docs_routes(state: AppState) -> ApiRouter {
    aide::generate::infer_responses(true);

    let router = ApiRouter::new()
        .api_route("/", get(|| async { Redirect::to("/docs/api") }))
        .api_route(
            "/api",
            get_with(
                Swagger::new("/docs/private/api.json")
                    .with_title("GeoNames FST API")
                    .axum_handler(),
                |op| op.description("Get the OpenAPI documentation for the GeoNames FST API"),
            ),
        )
        .route("/private/api.json", get(serve_docs))
        .with_state(state);

    // Afterwards we disable response inference because
    // it might be incorrect for other routes.
    aide::generate::infer_responses(false);

    router
}

async fn serve_docs(Extension(api): Extension<OpenApi>) -> impl IntoApiResponse {
    Json(api).into_response()
}

#[derive(serde::Serialize, schemars::JsonSchema)]
pub(crate) struct DocResults<T> {
    results: Vec<T>,
}

#[derive(serde::Serialize, schemars::JsonSchema)]
pub(crate) struct DocError {
    error: String,
}
