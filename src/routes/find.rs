use aide::axum::IntoApiResponse;
use axum::extract::State;
use axum::{http::StatusCode, Json};
use schemars::JsonSchema;
use serde::Deserialize;

use super::{filter_results, FilterResults, Response};
use crate::AppState;

use crate::geonames::data::GeoNamesSearchResult;

fn _schemars_default_filter_class_t() -> Option<FilterResults> {
    Some(FilterResults {
        feature_class: Some("T".to_string()),
        feature_code: None,
        country_code: Some("DE".to_string()),
    })
}
#[derive(Deserialize, JsonSchema)]
pub(crate) struct RequestOptsFind {
    #[schemars(default = "_schemars_default_filter_class_t")]
    pub filter: Option<FilterResults>,
}

fn _schemars_default_query() -> String {
    "Feldberg".to_string()
}
#[derive(Deserialize, JsonSchema)]
pub(crate) struct RequestFind {
    /// The search query (name of the GeoNames entity).
    #[validate(length(min = 1))]
    #[schemars(default = "_schemars_default_query")]
    pub query: String,

    #[serde(flatten)]
    pub opts: RequestOptsFind,
}

pub(crate) async fn find(
    State(state): State<AppState>,
    Json(request): Json<RequestFind>,
) -> impl IntoApiResponse {
    if request.query.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(Response::Error("Empty query".to_string())),
        );
    }

    let results: Vec<GeoNamesSearchResult> =
        filter_results(state.searcher.get(&request.query), &request.opts.filter);

    (StatusCode::OK, Json(Response::Results(results)))
}
