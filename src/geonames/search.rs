use std::str::FromStr;

use aide::axum::IntoApiResponse;
use axum::extract::State;
use axum::{http::StatusCode, Json};
use regex_automata::dfa::dense::DFA;
use regex_automata::dfa::{dense, Automaton as RegexAutomaton};
use regex_automata::util::primitives::StateID;
use regex_automata::Input;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::geonames::{filter_results, FilterResults, Response, _schemars_default_filter};
use crate::AppState;

use super::data::GeoNamesSearchResult;

fn _schemars_default_query() -> String {
    "Feldberg".to_string()
}
fn _schemars_default_filter_class_t() -> Option<FilterResults> {
    Some(FilterResults {
        feature_class: Some("T".to_string()),
        feature_code: None,
        country_code: Some("DE".to_string()),
    })
}
#[derive(Deserialize, JsonSchema)]
pub(crate) struct RequestFind {
    /// The search query (name of the GeoNames entity).
    #[validate(length(min = 1))]
    #[schemars(default = "_schemars_default_query")]
    pub query: String,
    #[schemars(default = "_schemars_default_filter_class_t")]
    pub filter: Option<FilterResults>,
}

pub(crate) async fn find(
    State(state): State<AppState>,
    Json(request): Json<RequestFind>,
) -> impl IntoApiResponse {
    if request.query.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(Response::Error("Empty query".to_string())),
        );
    }

    let results: Vec<GeoNamesSearchResult> =
        filter_results(state.searcher.get(&request.query), &request.filter);

    (StatusCode::OK, Json(Response::Results(results)))
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
        state
            .map(|state| self.dfa.is_match_state(self.dfa.next_eoi_state(state)))
            .unwrap_or(false)
    }

    fn accept(&self, state: &Self::State, byte: u8) -> Self::State {
        state.and_then(|state| Some(self.dfa.next_state(state, byte)))
    }
}

fn _schemars_default_regex() -> String {
    "^Frankfurt.*".to_string()
}
#[derive(Deserialize, JsonSchema)]
pub(crate) struct RequestRegex {
    /// The regular expression to match against the GeoNames entities.
    #[validate(length(min = 1))]
    #[schemars(default = "_schemars_default_regex")]
    pub regex: String,
    #[schemars(
        default = "_schemars_default_filter",
        skip_serializing_if = "Option::is_none"
    )]
    pub filter: Option<FilterResults>,
}

pub(crate) async fn regex(
    State(state): State<AppState>,
    Json(request): Json<RequestRegex>,
) -> impl IntoApiResponse {
    if request.regex.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(Response::Error("Empty query".to_string())),
        );
    }

    let dfa = RegexSearchAutomaton::from_str(&request.regex);
    if let Ok(query) = dfa {
        let results = filter_results(state.searcher.search(query), &request.filter);

        (StatusCode::OK, Json(Response::Results(results)))
    } else {
        let e = dfa.unwrap_err();

        (
            StatusCode::BAD_REQUEST,
            Json(Response::Error(format!("RegexError: {:?}", e).to_string())),
        )
    }
}
