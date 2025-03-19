use aide::axum::IntoApiResponse;
use axum::extract::State;
use axum::{http::StatusCode, Json};
use fst::automaton::Str;
use fst::Automaton;
use schemars::JsonSchema;
use serde::Deserialize;

use super::{filter_results, FilterResults, Response, _schemars_default_filter};
use crate::AppState;

fn _schemars_default_max_dist() -> Option<u32> {
    None
}
#[derive(Deserialize, JsonSchema)]
pub(crate) struct RequestOptsStartsWith {
    /// Filter results by Levenshtein distance. Omit or set to `null` to disable filtering.
    #[schemars(default = "_schemars_default_max_dist")]
    pub max_dist: Option<u32>,
    #[schemars(default = "_schemars_default_filter")]
    pub filter: Option<FilterResults>,
}

fn _schemars_default_query() -> String {
    "Frankfurt".to_string()
}
#[derive(Deserialize, JsonSchema)]
pub(crate) struct RequestStartsWith {
    /// The search query (name of the GeoNames entity).
    #[validate(length(min = 1))]
    #[schemars(default = "_schemars_default_query")]
    pub query: String,

    #[serde(flatten)]
    pub opts: RequestOptsStartsWith,
}

pub(crate) async fn starts_with(
    State(state): State<AppState>,
    Json(request): Json<RequestStartsWith>,
) -> impl IntoApiResponse {
    if request.query.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(Response::Error("Empty query".to_string())),
        );
    }

    let query = Str::new(&request.query).starts_with();

    let results = state
        .searcher
        .search_with_dist(query, &request.query, &request.opts.max_dist);
    let results = filter_results(results, &request.opts.filter);

    (StatusCode::OK, Json(Response::ResultsWithDist(results)))
}
