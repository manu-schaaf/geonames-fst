use aide::axum::IntoApiResponse;
use aide::transform::TransformOperation;
use axum::extract::State;
use axum::{http::StatusCode, Json};
use fst::automaton::Subsequence;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_aux::prelude::*;

use super::docs::{DocError, DocResults};
use super::{filter_results, FilterResults, Response, _schemars_default_filter};
use crate::geonames::data::GeoNamesSearchResultWithDist;
use crate::AppState;

#[derive(Deserialize, JsonSchema)]
pub(crate) struct RequestOptsFuzzy {
    /// Filter results by Levenshtein distance. Omit or set to `0` to disable filtering.
    #[serde(
        default = "default_u32::<0>",
        deserialize_with = "deserialize_number_from_string"
    )]
    pub max_dist: u32,
    #[schemars(default = "_schemars_default_filter")]
    pub filter: Option<FilterResults>,
}

fn _schemars_default_fuzzy_query() -> String {
    "FrnkfraMain".to_string()
}
#[derive(Deserialize, JsonSchema)]
pub(crate) struct RequestFuzzy {
    /// The search query (name of the GeoNames entity).
    #[validate(length(min = 1))]
    #[schemars(default = "_schemars_default_fuzzy_query")]
    pub query: String,

    #[serde(flatten)]
    pub opts: RequestOptsFuzzy,
}

pub(crate) async fn fuzzy(
    State(state): State<AppState>,
    Json(request): Json<RequestFuzzy>,
) -> impl IntoApiResponse {
    if request.query.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(Response::Error("Empty query".to_string())),
        );
    }

    let query = Subsequence::new(&request.query);

    let results =
        state
            .searcher
            .search_with_dist(query, &request.query, Some(request.opts.max_dist));
    let results = filter_results(results, &request.opts.filter);

    (StatusCode::OK, Json(Response::Results(results)))
}

pub(crate) fn fuzzy_docs(op: TransformOperation) -> TransformOperation {
    op.description(
        "Find all GeoNames entries that match the fuzzy search query with a maximum edit distance.",
    )
    .response::<200, Json<DocResults<GeoNamesSearchResultWithDist>>>()
    .response_with::<400, Json<DocError>, _>(|t| t.description("The query was empty."))
}
