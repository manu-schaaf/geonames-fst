use crate::utils::AppState;

use std::f32;
use std::str::FromStr;
use std::sync::Arc;

use axum::response::IntoResponse;
use axum::{http::StatusCode, Json};
use fst::automaton::{Str, Subsequence};
use fst::Automaton;
use fst::{automaton::Levenshtein, IntoStreamer, Streamer};
use levenshtein::levenshtein as levenshtein_dist;
use regex_automata::util::primitives::StateID;
use regex_automata::Input;
use serde::{Deserialize, Serialize};

use regex_automata::dfa::dense::DFA;
use regex_automata::dfa::{dense, Automaton as RegexAutomaton};

use crate::utils::GeoNamesData;

#[derive(Deserialize)]
pub(crate) struct RequestString {
    query: String,
    distance: Option<u32>,
}

#[derive(Serialize)]
enum Response {
    #[serde(rename = "results")]
    Results(Vec<ResponseInner>),
    #[serde(rename = "error")]
    Error(String),
}

#[derive(Serialize)]
pub(crate) struct ResponseInner {
    key: String,
    name: String,
    latitude: f32,
    longitude: f32,
    feature_class: String,
    feature_code: String,
    country_code: String,
    distance: usize,
}

impl ResponseInner {
    pub fn new(key: &str, dist: usize, gnd: &GeoNamesData) -> Self {
        ResponseInner {
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

pub(crate) async fn starts_with(
    Json(request): Json<RequestString>,
    state: Arc<AppState>,
) -> impl IntoResponse {
    if request.query.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(Response::Error("Empty query".to_string())),
        );
    }

    let query = Str::new(&request.query).starts_with();

    let mut stream = state.as_ref().map.search(&query).into_stream();
    let mut results = Vec::new();
    while let Some((key, gnd)) = stream.next() {
        let key = String::from_utf8_lossy(key).to_string();

        let dist = levenshtein_dist(&request.query, &key);
        if let Some(distance) = request.distance {
            if dist > (distance as usize) {
                continue;
            }
        }

        let gnd: &GeoNamesData = state.as_ref().data_store.get(&gnd).unwrap();
        results.push(ResponseInner::new(&key, dist, gnd));
    }
    results.sort_by(|a, b| a.distance.cmp(&b.distance));

    (StatusCode::OK, Json(Response::Results(results)))
}

pub(crate) async fn fuzzy(
    Json(request): Json<RequestString>,
    state: Arc<AppState>,
) -> impl IntoResponse {
    if request.query.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(Response::Error("Empty query".to_string())),
        );
    }

    let query = Subsequence::new(&request.query);

    let mut stream = state.as_ref().map.search(&query).into_stream();
    let mut results = Vec::new();
    while let Some((key, gnd)) = stream.next() {
        let key = String::from_utf8_lossy(key).to_string();

        let dist = levenshtein_dist(&request.query, &key);
        if let Some(distance) = request.distance {
            if dist > (distance as usize) {
                continue;
            }
        }

        let gnd: &GeoNamesData = state.as_ref().data_store.get(&gnd).unwrap();
        results.push(ResponseInner::new(&key, dist, gnd));
    }
    results.sort_by(|a, b| a.distance.cmp(&b.distance));

    (StatusCode::OK, Json(Response::Results(results)))
}

#[derive(Deserialize)]
pub(crate) struct RequestWithLimit {
    query: String,
    distance: Option<u32>,
    limit: Option<usize>,
}

pub(crate) async fn levenshtein(
    Json(request): Json<RequestWithLimit>,
    state: Arc<AppState>,
) -> impl IntoResponse {
    let distance = request.distance.unwrap_or(1);

    let query = if let Some(limit) = request.limit {
        Levenshtein::new_with_limit(&request.query, distance, limit)
    } else {
        Levenshtein::new(&request.query, distance)
    };

    if let Ok(query) = query {
        let mut results = Vec::new();
        let mut stream = state.as_ref().map.search_with_state(&query).into_stream();
        while let Some((key, gnd, _)) = stream.next() {
            let key = String::from_utf8_lossy(key).to_string();
            let dist = levenshtein_dist(&request.query, &key);
            let gnd: &GeoNamesData = state.as_ref().data_store.get(&gnd).unwrap();
            results.push(ResponseInner::new(&key, dist, gnd));
        }
        results.sort_by(|a, b| a.distance.cmp(&b.distance));

        (StatusCode::OK, Json(Response::Results(results)))
    } else {
        let error = query.unwrap_err();

        (
            StatusCode::BAD_REQUEST,
            Json(Response::Error(
                format!("LevenshteinError: {:?}", error).to_string(),
            )),
        )
    }
}

#[derive(Debug)]
struct RegexSearchAutomaton {
    dfa: DFA<Vec<u32>>,
    start_state: StateID,
}

impl FromStr for RegexSearchAutomaton {
    type Err = anyhow::Error;

    fn from_str(query: &str) -> Result<Self, Self::Err> {
        let dfa = dense::DFA::new(query)?;
        let start_state = dfa.start_state_forward(&Input::new(query))?;
        Ok(RegexSearchAutomaton { dfa, start_state })
    }
}

impl fst::Automaton for RegexSearchAutomaton {
    type State = Option<StateID>;

    #[inline]
    fn start(&self) -> Option<StateID> {
        Some(self.start_state)
    }

    fn is_match(&self, state: &Self::State) -> bool {
        // println!("is_match: {:?}", state);
        state
            .map(|state| self.dfa.is_match_state(self.dfa.next_eoi_state(state)))
            .unwrap_or(false)
    }

    fn accept(&self, state: &Self::State, byte: u8) -> Self::State {
        // println!("accept: {:?}, {:?}", state, byte);
        state.and_then(|state| Some(self.dfa.next_state(state, byte)))
    }
}

pub(crate) async fn regex(
    Json(request): Json<RequestString>,
    state: Arc<AppState>,
) -> impl IntoResponse {
    if request.query.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(Response::Error("Empty query".to_string())),
        );
    }

    let dfa = RegexSearchAutomaton::from_str(&request.query);
    if let Ok(query) = dfa {
        let mut stream = state.as_ref().map.search(&query).into_stream();
        let mut results = Vec::new();
        while let Some((key, gnd)) = stream.next() {
            let key = String::from_utf8_lossy(key).to_string();

            let dist = levenshtein_dist(&request.query, &key);
            if let Some(distance) = request.distance {
                if dist > (distance as usize) {
                    continue;
                }
            }

            let gnd: &GeoNamesData = state.as_ref().data_store.get(&gnd).unwrap();
            results.push(ResponseInner::new(&key, dist, gnd));
        }
        results.sort_by(|a, b| a.distance.cmp(&b.distance));

        (StatusCode::OK, Json(Response::Results(results)))
    } else {
        let e = dfa.unwrap_err();

        (
            StatusCode::BAD_REQUEST,
            Json(Response::Error(format!("RegexError: {:?}", e).to_string())),
        )
    }
}
