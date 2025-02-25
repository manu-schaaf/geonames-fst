pub mod search;
pub mod search_with_dist;
pub mod utils;

use std::sync::Arc;

use aide::axum::routing::{get, post_with};
use clap::{command, Parser};
use schemars::JsonSchema;
use search::get_geoname;
use serde::Serialize;

use aide::{
    axum::{ApiRouter, IntoApiResponse},
    openapi::{Info, OpenApi},
};
use axum::{response::IntoResponse, Extension, Json};

use utils::{build_searcher, GeoNamesSearchResult, GeoNamesSearchResultWithDist, GeoNamesSearcher};

#[derive(Clone)]
struct AppState {
    searcher: Arc<GeoNamesSearcher>,
}

#[derive(Serialize, JsonSchema)]
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

async fn serve_api(Extension(api): Extension<OpenApi>) -> impl IntoApiResponse {
    Json(api).into_response()
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let app_state = AppState {
        searcher: Arc::new(build_searcher(args.paths, args.alternate, args.languages)?),
    };

    let app = ApiRouter::new()
        .api_route("/get", post_with(get_geoname, |o| o))
        .api_route(
            "/starts_with",
            post_with(crate::search_with_dist::starts_with, |o| o),
        )
        .api_route("/fuzzy", post_with(crate::search_with_dist::fuzzy, |o| o))
        .api_route(
            "/levenshtein",
            post_with(crate::search_with_dist::levenshtein, |o| o),
        )
        .api_route("/regex", post_with(crate::search::regex, |o| o))
        .route("/api.json", get(serve_api));

    let mut api = OpenApi {
        info: Info {
            description: Some("GeoNames FST API".to_string()),
            ..Info::default()
        },
        ..OpenApi::default()
    };

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await?;
    axum::serve(
        listener,
        app.finish_api(&mut api)
            .layer(Extension(Arc::new(api)))
            .with_state(app_state)
            .into_make_service(),
    )
    .await?;
    Ok(())
}
