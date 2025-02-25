pub mod search;
pub mod search_with_dist;
pub mod utils;

use std::sync::Arc;

use axum::{routing::post, Router};
use clap::{command, Parser};
use serde::Serialize;

use utils::{build_searcher, GeoNamesSearchResult, GeoNamesSearchResultWithDist, GeoNamesSearcher};

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

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[clap(help = "Paths to GeoNames files")]
    paths: Vec<String>,
    #[clap(short, long, help = "Paths to `alternateNames` files")]
    alternate: Option<Vec<String>>,
    #[clap(
        short,
        long,
        help = "Languages to consider for the alternatives.",
        default_value = ",de,ger",
        value_delimiter = ','
    )]
    languages: Option<Vec<String>>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    println!("args: {:?}", args);

    let app_state: Arc<AppState> = Arc::new(AppState {
        searcher: build_searcher(args.paths, args.alternate, args.languages)?,
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
