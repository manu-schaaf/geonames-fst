pub mod search;
pub mod search_with_dist;
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
            post(move |body| crate::search::get(body, state))
        })
        .route("/starts_with", {
            let state = Arc::clone(&app_state);
            post(move |body| crate::search_with_dist::starts_with(body, state))
        })
        .route("/fuzzy", {
            let state = Arc::clone(&app_state);
            post(move |body| crate::search_with_dist::fuzzy(body, state))
        })
        .route("/levenshtein", {
            let state = Arc::clone(&app_state);
            post(move |body| crate::search_with_dist::levenshtein(body, state))
        })
        .route("/regex", {
            let state = Arc::clone(&app_state);
            post(move |body| crate::search::regex(body, state))
        });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
