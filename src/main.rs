pub mod utils;

use std::collections::HashMap;
use std::f32;
use std::sync::Arc;

use axum::response::IntoResponse;
use fst::{automaton::Levenshtein, IntoStreamer, Map, Streamer};
use levenshtein::levenshtein;

use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use utils::{build_fst, GeoNamesData};

struct AppState {
    map: Map<Vec<u8>>,
    data_store: HashMap<u64, GeoNamesData>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let (map, data_store) = build_fst()?;
    let app_state = Arc::new(AppState { map, data_store });
    let app = Router::new().route(
        "/levenshtein",
        post(move |body| route_levenshtein(body, Arc::clone(&app_state))),
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Serialize)]
enum ResponseLevenshtein {
    #[serde(rename = "results")]
    Results(Vec<ResponseLevenshteinInner>),
    #[serde(rename = "error")]
    Error(String),
}

#[derive(Serialize)]
struct ResponseLevenshteinInner {
    key: String,
    name: String,
    latitude: f32,
    longitude: f32,
    feature_class: String,
    feature_code: String,
    country_code: String,
    distance: usize,
}

impl ResponseLevenshteinInner {
    pub fn new(key: &str, dist: usize, gnd: &GeoNamesData) -> Self {
        ResponseLevenshteinInner {
            key: key.to_string(),
            name: gnd.name.clone(),
            latitude: gnd.latitude,
            longitude: gnd.longitude,
            feature_class: gnd.feature_class.clone(),
            feature_code: gnd.feature_code.clone(),
            country_code: gnd.country_code.clone(),
            distance: dist,
        }
    }
}

#[derive(Deserialize)]
struct RequestLevenshtein {
    query: String,
    distance: Option<u32>,
    limit: Option<usize>,
}

async fn route_levenshtein(
    Json(request): Json<RequestLevenshtein>,
    state: Arc<AppState>,
) -> impl IntoResponse {
    let raw_query = request.query.trim();
    let distance = request.distance.unwrap_or(1);

    let query = if let Some(limit) = request.limit {
        Levenshtein::new_with_limit(&raw_query, distance, limit)
    } else {
        Levenshtein::new(&raw_query, distance)
    };

    if let Ok(query) = query {
        let mut results = Vec::new();
        let mut stream = state.as_ref().map.search_with_state(&query).into_stream();
        while let Some((key, val, _)) = stream.next() {
            let key = String::from_utf8_lossy(key).to_string();
            let dist = levenshtein(&raw_query, &key);
            let val: &GeoNamesData = state.as_ref().data_store.get(&val).unwrap();
            results.push(ResponseLevenshteinInner::new(&key, dist, val));
        }
        results.sort_by(|a, b| a.distance.cmp(&b.distance));

        return (StatusCode::OK, Json(ResponseLevenshtein::Results(results)));
    } else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ResponseLevenshtein::Error("Invalid query".to_string())),
        );
    }
}
