pub mod search;
pub mod utils;

use std::sync::Arc;

use axum::{routing::post, Router};

use utils::{build_fst, AppState};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let app_state: Arc<AppState> = Arc::new(build_fst()?);
    let app = Router::new()
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
        });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
