mod documentation;
mod process;

use aide::axum::{
    routing::{get_with, post_with},
    ApiRouter,
};
use axum::Json;
use tower_http::services::ServeFile;

use crate::duui::documentation::{v1_documentation, Documentation};
use crate::duui::process::{v1_process, v1_process_docs};
use crate::AppState;

pub(crate) fn duui_routes(state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route("/process", post_with(v1_process, v1_process_docs))
        .route_service(
            "/communication_layer",
            ServeFile::new("resources/communication_layer.lua"),
        )
        .route_service("/typesystem", ServeFile::new("resources/typesystem.xml"))
        .api_route(
            "/documentation",
            get_with(v1_documentation, |op| {
                op.description("DUUI documentation")
                    .response::<200, Json<Documentation>>()
            }),
        )
        .with_state(state)
}
