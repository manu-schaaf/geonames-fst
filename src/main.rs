pub mod regex;
pub mod search;
pub mod utils;

use std::sync::Arc;

use axum::{routing::post, Router};
use serde::Serialize;

use utils::{build_fst, GeoNamesSearchResult, GeoNamesSearchResultWithDist, GeoNamesSearcher};

struct AppState {
    searcher: GeoNamesSearcher,
}

#[derive(Serialize)]
pub(crate) enum Response {
    #[serde(rename = "results")]
    Results(Vec<GeoNamesSearchResult>),
    #[serde(rename = "results")]
    ResultsWithDist(Vec<GeoNamesSearchResultWithDist>),
    #[serde(rename = "error")]
    Error(String),
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let app_state: Arc<AppState> = Arc::new(AppState {
        searcher: build_fst()?,
    });
    let app = Router::new()
        .route("/get", {
            let state = Arc::clone(&app_state);
            post(move |body| crate::regex::get(body, state))
        })
        .route("/starts_with", {
            let state = Arc::clone(&app_state);
            post(move |body| crate::search::starts_with(body, state))
        })
        .route("/fuzzy", {
            let state = Arc::clone(&app_state);
            post(move |body| crate::search::fuzzy(body, state))
        })
        .route("/levenshtein", {
            let state = Arc::clone(&app_state);
            post(move |body| crate::search::levenshtein(body, state))
        })
        .route("/regex", {
            let state = Arc::clone(&app_state);
            post(move |body| crate::regex::regex(body, state))
        });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
