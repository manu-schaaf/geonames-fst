use crate::{AppState, Response};

use std::sync::Arc;

use axum::response::IntoResponse;
use axum::{http::StatusCode, Json};
use fst::automaton::Levenshtein;
use fst::automaton::{Str, Subsequence};
use fst::Automaton;
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct RequestWithDist {
    pub query: String,
    pub distance: Option<u32>,
}

pub(crate) async fn starts_with(
    Json(request): Json<RequestWithDist>,
    state: Arc<AppState>,
) -> impl IntoResponse {
    if request.query.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(Response::Error("Empty query".to_string())),
        );
    }

    let query = Str::new(&request.query).starts_with();

    let results =
        state
            .as_ref()
            .searcher
            .search_with_dist(query, &request.query, &request.distance);

    (StatusCode::OK, Json(Response::ResultsWithDist(results)))
}

pub(crate) async fn fuzzy(
    Json(request): Json<RequestWithDist>,
    state: Arc<AppState>,
) -> impl IntoResponse {
    if request.query.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(Response::Error("Empty query".to_string())),
        );
    }

    let query = Subsequence::new(&request.query);

    let results =
        state
            .as_ref()
            .searcher
            .search_with_dist(query, &request.query, &request.distance);

    (StatusCode::OK, Json(Response::ResultsWithDist(results)))
}

#[derive(Deserialize)]
pub(crate) struct RequestWithLimit {
    query: String,
    distance: Option<u32>,
    limit: Option<usize>,
}

pub(crate) async fn levenshtein(
    Json(request): Json<RequestWithLimit>,
    state: Arc<AppState>,
) -> impl IntoResponse {
    if request.query.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(Response::Error("Empty query".to_string())),
        );
    }

    let distance = request.distance.unwrap_or(1);

    let query = if let Some(limit) = request.limit {
        Levenshtein::new_with_limit(&request.query, distance, limit)
    } else {
        Levenshtein::new(&request.query, distance)
    };

    if let Ok(query) = query {
        let results =
            state
                .as_ref()
                .searcher
                .search_with_dist(query, &request.query, &request.distance);
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
