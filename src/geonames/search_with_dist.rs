use aide::axum::IntoApiResponse;
use axum::extract::State;
use axum::{http::StatusCode, Json};
use fst::automaton::Levenshtein;
use fst::automaton::{Str, Subsequence};
use fst::Automaton;
use schemars::JsonSchema;
use serde::Deserialize;

use super::Response;
use crate::AppState;

fn _schemars_default_query() -> String {
    "Frankfurt".to_string()
}

fn _schemars_default_max_dist() -> Option<u32> {
    None
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct RequestStartsWith {
    /// The search query (name of the GeoNames entity).
    #[validate(length(min = 1))]
    #[schemars(default = "_schemars_default_query")]
    pub query: String,
    /// Filter results by Levenshtein distance. Omit or set to `null` to disable filtering.
    #[schemars(default = "_schemars_default_max_dist")]
    pub max_dist: Option<u32>,
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
        .search_with_dist(query, &request.query, &request.max_dist);

    (StatusCode::OK, Json(Response::ResultsWithDist(results)))
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
    /// Filter results by Levenshtein distance. Omit or set to `null` to disable filtering.
    #[schemars(default = "_schemars_default_max_dist")]
    pub max_dist: Option<u32>,
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

    let results = state
        .searcher
        .search_with_dist(query, &request.query, &request.max_dist);

    (StatusCode::OK, Json(Response::ResultsWithDist(results)))
}

fn _schemars_default_levenshtein_query() -> String {
    "Frxnkfxrt".to_string()
}

fn _schemars_default_max_dist_one() -> Option<u32> {
    Some(2)
}

fn _schemars_default_state_limit() -> Option<usize> {
    Some(10000)
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct RequestLevenshtein {
    /// The search query (name of the GeoNames entity).
    #[validate(length(min = 1))]
    #[schemars(default = "_schemars_default_levenshtein_query")]
    pub query: String,
    /// Maximum Levenshtein distance. Defaults to 1.
    #[schemars(default = "_schemars_default_max_dist_one")]
    pub max_dist: Option<u32>,
    /// Limit the number of states to search. Defaults to 10000. Long queries or high `max_dist` values may require increasing this limit.
    #[schemars(default = "_schemars_default_state_limit")]
    state_limit: Option<usize>,
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

    let distance = request.max_dist.unwrap_or(1);

    let query = if let Some(state_limit) = request.state_limit {
        Levenshtein::new_with_limit(&request.query, distance, state_limit)
    } else {
        Levenshtein::new(&request.query, distance)
    };

    if let Ok(query) = query {
        let results = state
            .searcher
            .search_with_dist(query, &request.query, &request.max_dist);
        (StatusCode::OK, Json(Response::ResultsWithDist(results)))
    } else {
        let error = query.unwrap_err();

        (
            StatusCode::NOT_ACCEPTABLE,
            Json(Response::Error(
                format!("LevenshteinError: {:?}", error).to_string(),
            )),
        )
    }
}
