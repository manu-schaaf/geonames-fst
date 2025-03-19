pub mod geonames;
pub mod routes;

use std::sync::Arc;

use aide::axum::routing::get;
use aide::{axum::ApiRouter, openapi::OpenApi};
use axum::response::Redirect;
use axum::Extension;
use clap::{command, Parser};
use routes::geonames_routes;

use crate::geonames::searcher::GeoNamesSearcher;
use crate::routes::docs::docs_routes;

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
        help = "Languages to consider for the alternative names.",
        default_value = ",de,deu,ger,de-DE,de-AT,de-CH",
        value_delimiter = ','
    )]
    languages: Vec<String>,
    #[clap(long, default_value = "0.0.0.0")]
    host: String,
    #[clap(long, default_value = "8000")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let mut paths = Vec::new();
    for path in args.paths.iter() {
        if std::fs::metadata(path)?.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    paths.push(entry.path().to_string_lossy().to_string());
                }
            }
        } else {
            paths.push(path.to_string());
        }
    }

    let alternate = if let Some(alternate) = args.alternate.as_ref() {
        let mut alternate_paths = Vec::new();
        for path in alternate.iter() {
            if std::fs::metadata(path)?.is_dir() {
                for entry in std::fs::read_dir(path)? {
                    let entry = entry?;
                    if entry.file_type()?.is_file() {
                        alternate_paths.push(entry.path().to_string_lossy().to_string());
                    }
                }
            } else {
                alternate_paths.push(path.to_string());
            }
        }
        Some(alternate_paths)
    } else {
        None
    };

    let app_state = AppState {
        searcher: Arc::new(GeoNamesSearcher::build(
            paths,
            alternate,
            Some(args.languages),
        )?),
    };

    let mut api = OpenApi::default();

    let app = ApiRouter::new()
        .nest_api_service("/geonames", geonames_routes(app_state.clone()))
        .nest_api_service("/docs", docs_routes(app_state.clone()))
        .api_route("/", get(|| async { Redirect::to("/docs/api") }))
        .finish_api(&mut api)
        .layer(Extension(api))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", args.host, args.port)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
