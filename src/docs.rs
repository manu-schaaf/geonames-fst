use aide::axum::{routing::get_with, ApiRouter, IntoApiResponse};

use crate::AppState;
use aide::swagger::Swagger;
use aide::{axum::routing::get, openapi::OpenApi};
use axum::{response::IntoResponse, Extension, Json};

pub(crate) fn docs_routes(state: AppState) -> ApiRouter {
    aide::generate::infer_responses(true);

    let router = ApiRouter::new()
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
        .with_state(state)
        ;

    // Afterwards we disable response inference because
    // it might be incorrect for other routes.
    aide::generate::infer_responses(false);

    router
}

async fn serve_docs(Extension(api): Extension<OpenApi>) -> impl IntoApiResponse {
    Json(api).into_response()
}
