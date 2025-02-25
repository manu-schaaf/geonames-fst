use crate::{AppState, Response};

use aide::axum::IntoApiResponse;
use axum::extract::State;
use axum::{http::StatusCode, Json};
use fst::automaton::Levenshtein;
use fst::automaton::{Str, Subsequence};
use fst::Automaton;
use serde::Deserialize;

use schemars::JsonSchema;

#[derive(Deserialize, JsonSchema)]
pub(crate) struct RequestWithDist {
    pub query: String,
    pub max_dist: Option<u32>,
}

pub(crate) async fn starts_with(
    State(state): State<AppState>,
    Json(request): Json<RequestWithDist>,
) -> impl IntoApiResponse {
    if request.query.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(Response::Error("Empty query".to_string())),
        );
    }

    let query = Str::new(&request.query).starts_with();

    let results =
        state
            .searcher
            .search_with_dist(query, &request.query, &request.max_dist);

    (StatusCode::OK, Json(Response::ResultsWithDist(results)))
}

pub(crate) async fn fuzzy(
    State(state): State<AppState>,
    Json(request): Json<RequestWithDist>,
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
            .search_with_dist(query, &request.query, &request.max_dist);

    (StatusCode::OK, Json(Response::ResultsWithDist(results)))
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct RequestWithLimit {
    query: String,
    max_dist: Option<u32>,
    state_limit: Option<usize>,
}

pub(crate) async fn levenshtein(
    State(state): State<AppState>,
    Json(request): Json<RequestWithLimit>,
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
        let results =
            state
                .searcher
                .search_with_dist(query, &request.query, &request.max_dist);
        (StatusCode::OK, Json(Response::ResultsWithDist(results)))
    } else {
        let error = query.unwrap_err();

        (
            StatusCode::BAD_REQUEST,
            Json(Response::Error(
                format!("LevenshteinError: {:?}", error).to_string(),
            )),
        )
    }
}
