use aide::axum::IntoApiResponse;
use aide::transform::TransformOperation;
use axum::extract::State;
use axum::{http::StatusCode, Json};
use fst::automaton::{Levenshtein, LevenshteinError};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_aux::prelude::*;

use super::docs::{DocError, DocResults};
use super::{filter_results, FilterResults, Response, _schemars_default_filter};
use crate::geonames::data::GeoNamesSearchResultWithDist;
use crate::geonames::searcher::GeoNamesSearcher;
use crate::AppState;

fn _schemars_default_max_dist() -> u32 {
    2
}
fn _default_state_limit() -> usize {
    10000
}
#[derive(Deserialize, JsonSchema)]
pub(crate) struct RequestOptsLevenshtein {
    /// Maximum Levenshtein distance. Defaults to 1.
    #[serde(
        default = "default_u32::<1>",
        deserialize_with = "deserialize_number_from_string"
    )]
    #[schemars(default = "_schemars_default_max_dist")]
    pub max_dist: u32,
    /// Limit the number of states to search. Defaults to 10000. Long queries or high `max_dist` values may require increasing this limit.
    #[serde(
        default = "_default_state_limit",
        deserialize_with = "deserialize_number_from_string"
    )]
    pub state_limit: usize,
    #[schemars(default = "_schemars_default_filter")]
    pub filter: Option<FilterResults>,
}

fn _schemars_default_levenshtein_query() -> String {
    "Frxnkfxrt".to_string()
}
#[derive(Deserialize, JsonSchema)]
pub(crate) struct RequestLevenshtein {
    /// The search query (name of the GeoNames entity).
    #[validate(length(min = 1))]
    #[schemars(default = "_schemars_default_levenshtein_query")]
    pub query: String,

    #[serde(flatten)]
    pub opts: RequestOptsLevenshtein,
}

pub(crate) async fn levenshtein(
    State(state): State<AppState>,
    Json(request): Json<RequestLevenshtein>,
) -> impl IntoApiResponse {
    if request.query.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(Response::Error("Empty query".to_string())),
        );
    }

    match levenshtein_inner(
        &state.searcher,
        &request.query,
        request.opts.state_limit,
        request.opts.max_dist,
        &request.opts.filter,
    ) {
        Ok(results) => (StatusCode::OK, Json(Response::Results(results))),
        Err(error) => (
            StatusCode::NOT_ACCEPTABLE,
            Json(Response::Error(
                format!("LevenshteinError: {:?}", error).to_string(),
            )),
        ),
    }
}

pub(crate) fn levenshtein_inner(
    searcher: &GeoNamesSearcher,
    query: &str,
    state_limit: usize,
    max_dist: u32,
    filter: &Option<FilterResults>,
) -> Result<Vec<GeoNamesSearchResultWithDist>, LevenshteinError> {
    let levenshtein_query = Levenshtein::new_with_limit(query, max_dist, state_limit);
    match levenshtein_query {
        Ok(levenshtein_query) => Ok(filter_results(
            searcher.search_with_dist(levenshtein_query, query, None),
            filter,
        )),
        Err(error) => Err(error),
    }
}

pub(crate) fn levenshtein_docs(op: TransformOperation) -> TransformOperation {
    op.description("Find all GeoNames entries that match the Levenshtein search query with a maximum edit distance.<br><strong>NOTE:</strong> The Levenshtein search may consume a lot of memory and is thus capped to a maximum number of states of 10000 by default. If your search query exceeds this limit, you will recieve an error (406 Not Acceptable). The number of required states depends on the <code>max_dist</code>.<br><br><em>Use with caution!</em>")
        .response::<200, Json<DocResults<GeoNamesSearchResultWithDist>>>()
        .response_with::<400, Json<DocError>, _>(|t|t.description("The query was empty."))
        .response_with::<406, Json<DocError>, _>(|t| t.description("The search query exceeded the maximum number of states"))
}
