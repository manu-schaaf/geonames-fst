pub mod docs;
pub mod geonames;

use std::sync::Arc;

use aide::transform::TransformOpenApi;
use aide::{axum::ApiRouter, openapi::OpenApi};
use axum::Extension;
use clap::{command, Parser};

use crate::docs::docs_routes;
use crate::geonames::{geonames_routes, searcher::GeoNamesSearcher};

#[derive(Clone)]
struct AppState {
    searcher: Arc<GeoNamesSearcher>,
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

    let app_state = AppState {
        searcher: Arc::new(GeoNamesSearcher::build(
            args.paths,
            args.alternate,
            args.languages,
        )?),
    };

    let mut api = OpenApi::default();

    let app = ApiRouter::new()
        .nest_api_service("/geonames", geonames_routes(app_state.clone()))
        .nest_api_service("/docs", docs_routes(app_state.clone()))
        .finish_api_with(&mut api, api_docs)
        .layer(Extension(api))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn api_docs(api: TransformOpenApi) -> TransformOpenApi {
    api.title("GeoNames FST API")
}
