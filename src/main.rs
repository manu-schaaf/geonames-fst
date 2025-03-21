pub mod geonames;
pub mod routes;

#[cfg(feature = "duui")]
pub mod duui;

use std::sync::Arc;

use aide::axum::routing::get;
use aide::axum::IntoApiResponse;
use aide::{axum::ApiRouter, openapi::OpenApi};
use axum::http::StatusCode;
use axum::Extension;
use clap::{command, Parser};

#[cfg(feature = "geonames_routes")]
use routes::geonames_routes;
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::geonames::searcher::GeoNamesSearcher;
use crate::routes::docs::docs_routes;

#[cfg(feature = "duui")]
use crate::duui::duui_routes;

#[derive(Clone)]
struct AppState {
    searcher: Arc<GeoNamesSearcher>,
    #[cfg(feature = "duui")]
    languages: Option<Vec<String>>,
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

async fn get_version() -> impl IntoApiResponse {
    (
        StatusCode::OK,
        format!("{}:{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
    )
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                format!(
                    "{}=debug,tower_http=debug,axum::rejection=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

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
            alternate.as_ref(),
            Some(&args.languages),
        )?),
        #[cfg(feature = "duui")]
        languages: Some(args.languages),
    };

    let mut api = OpenApi::default();

    let app = ApiRouter::new()
        .route("/", get(get_version))
        .nest_api_service("/docs", docs_routes(app_state.clone()));

    #[cfg(feature = "geonames_routes")]
    let app = app.nest_api_service("/geonames", geonames_routes(app_state.clone()));

    #[cfg(feature = "duui")]
    let app = app.nest_api_service("/v1", duui_routes(app_state.clone()));

    let app = app
        .finish_api(&mut api)
        .layer(Extension(api))
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", args.host, args.port)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
