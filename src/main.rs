use std::sync::Arc;
use std::{collections::HashMap, fs::File};
use std::{f32, io};

use anyhow::anyhow;
use axum::extract::State;
use axum::response::IntoResponse;
use fst::{automaton::Levenshtein, IntoStreamer, Map, MapBuilder, Streamer};
use levenshtein::levenshtein;

use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GeoNamesData {
    name: String,
    latitude: f32,
    longitude: f32,
    feature_class: String,
    feature_code: String,
    country_code: String,
}

struct AppState {
    map: Map<Vec<u8>>,
    data_store: HashMap<u64, GeoNamesData>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let app_state = Arc::new(build_fst()?);
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
    pub fn new(
        key: &str,
        dist: usize,
        gnd: &GeoNamesData,
    ) -> Self {
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

fn build_fst() -> Result<AppState, anyhow::Error> {
    let mut search_terms: Vec<(String, u64)> = Vec::new();
    let mut data_store: HashMap<u64, GeoNamesData> = HashMap::new();
    parse_geonames_file(
        "data/geonames/DE.txt",
        // "data/geonames/allCountries.txt",
        &mut search_terms,
        &mut data_store,
    )?;
    println!("Read {} search terms", search_terms.len());
    search_terms.sort();
    search_terms.dedup_by(|(a, _), (b, _)| a == b);
    println!(
        "Sorted and deduplicated to {} search terms",
        search_terms.len()
    );
    let mut build = MapBuilder::memory();
    for (term, geoname_id) in search_terms {
        build.insert(term, geoname_id)?;
    }
    let bytes = build.into_inner()?;
    let num_bytes = bytes.len();
    let map = Map::new(bytes)?;
    println!("Built FST with {} bytes", num_bytes);
    Ok(AppState { map, data_store })
}

fn parse_geonames_file(
    path: &str,
    search_terms: &mut Vec<(String, u64)>,
    data_store: &mut HashMap<u64, GeoNamesData>,
) -> Result<(), anyhow::Error> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(io::BufReader::new(File::open(path)?));

    for row in rdr.records() {
        let record = row?;

        let geoname_id: u64 = record.get(0).ok_or(anyhow!("no geoname_id"))?.parse()?;
        let name: String = record.get(1).ok_or(anyhow!("no name"))?.to_string();

        let latitude: f32 = parse_float_else_nan(record.get(4));
        let longitude: f32 = parse_float_else_nan(record.get(5));
        let feature_class: String = record.get(6).unwrap_or("<missing>").to_string();
        let feature_code: String = record.get(7).unwrap_or("<missing>").to_string();
        let country_code: String = record.get(8).unwrap_or("<missing>").to_string();

        let data = GeoNamesData {
            name: name.clone(),
            latitude,
            longitude,
            feature_class,
            feature_code,
            country_code,
        };

        data_store.insert(geoname_id, data);

        search_terms.push((name, geoname_id));
        if let Some(alternate_names) = record.get(3) {
            for name in alternate_names.split(',') {
                if name.trim().is_empty() {
                    continue;
                }
                search_terms.push((name.trim().to_string(), geoname_id));
            }
        }
    }
    Ok(())
}

fn parse_float_else_nan(maybe_str: Option<&str>) -> f32 {
    if let Some(maybe_str) = maybe_str {
        maybe_str.trim().parse::<f32>().unwrap_or(f32::NAN)
    } else {
        f32::NAN
    }
}
