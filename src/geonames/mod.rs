pub mod data;
pub mod search;
pub mod search_with_dist;
pub mod searcher;
pub mod utils;

use aide::{
    axum::{routing::post_with, ApiRouter},
    transform::TransformOperation,
};
use axum::Json;

use crate::AppState;

pub(crate) fn geonames_routes(state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route("/find", post_with(search::find, find_docs))
        .api_route("/regex", post_with(search::regex, regex_docs))
        .api_route(
            "/starts_with",
            post_with(search_with_dist::starts_with, starts_with_docs),
        )
        .api_route("/fuzzy", post_with(search_with_dist::fuzzy, fuzzy_docs))
        .api_route(
            "/levenshtein",
            post_with(search_with_dist::levenshtein, levenshtein_docs),
        )
        .with_state(state)
}

#[derive(serde::Serialize, schemars::JsonSchema)]
pub(crate) enum Response {
    #[serde(rename = "results")]
    Results(Vec<data::GeoNamesSearchResult>),
    #[serde(rename = "results")]
    ResultsWithDist(Vec<data::GeoNamesSearchResultWithDist>),
    #[serde(rename = "error")]
    Error(String),
}

#[derive(serde::Serialize, schemars::JsonSchema)]
struct _DocResults {
    results: Vec<data::GeoNamesSearchResult>,
}

#[derive(serde::Serialize, schemars::JsonSchema)]
struct _DocResultsWithDist {
    results: Vec<data::GeoNamesSearchResultWithDist>,
}

#[derive(serde::Serialize, schemars::JsonSchema)]
struct _DocError {
    error: String,
}

fn find_docs(op: TransformOperation) -> TransformOperation {
    op.description("Find all GeoNames entries with the specified name")
        .response::<200, Json<_DocResults>>()
        .response_with::<400, Json<_DocError>, _>(|t| t.description("The query was empty."))
}

fn regex_docs(op: TransformOperation) -> TransformOperation {
    op.description("Find all GeoNames entries with the specified regex")
        .response::<200, Json<_DocResults>>()
        .response_with::<400, Json<_DocError>, _>(|t| t.description("The query was empty."))
}

fn starts_with_docs(op: TransformOperation) -> TransformOperation {
    op.description("Find all GeoNames entries that start with the specified string")
        .response::<200, Json<_DocResultsWithDist>>()
        .response_with::<400, Json<_DocError>, _>(|t| t.description("The query was empty."))
}

fn fuzzy_docs(op: TransformOperation) -> TransformOperation {
    op.description(
        "Find all GeoNames entries that match the fuzzy search query with a maximum edit distance",
    )
    .response::<200, Json<_DocResultsWithDist>>()
    .response_with::<400, Json<_DocError>, _>(|t| t.description("The query was empty."))
}

fn levenshtein_docs(op: TransformOperation) -> TransformOperation {
    op.description("Find all GeoNames entries that match the Levenshtein search query with a maximum edit distance.<br><strong>NOTE:</strong> The Levenshtein search may consume a lot of memory and is thus capped to a maximum number of states of 10000 by default. If your search query exceeds this limit, you will recieve an error (406 Not Acceptable). The number of required states depends on the <code>max_dist</code>. <em>Use with caution!</em>")
        .response::<200, Json<_DocResultsWithDist>>()
        .response_with::<400, Json<_DocError>, _>(|t|t.description("The query was empty."))
        .response_with::<406, Json<_DocError>, _>(|t| t.description("The search query exceeded the maximum number of states"))
}
