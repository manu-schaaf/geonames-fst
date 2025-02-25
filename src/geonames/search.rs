use crate::geonames::Response;
use crate::AppState;

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

fn _schemars_default_query() -> String {
    "Frankfurt".to_string()
}
#[derive(Deserialize, JsonSchema)]
pub(crate) struct Request {
    /// The search query (name of the GeoNames entity).
    #[validate(length(min = 1))]
    #[schemars(default = "_schemars_default_query")]
    pub query: String,
}

pub(crate) async fn find(
    State(state): State<AppState>,
    Json(request): Json<Request>,
) -> impl IntoApiResponse {
    if request.query.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(Response::Error("Empty query".to_string())),
        );
    }

    let results = state.searcher.get(&request.query);

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
        let results = state.searcher.search(query);

        (StatusCode::OK, Json(Response::Results(results)))
    } else {
        let e = dfa.unwrap_err();

        (
            StatusCode::BAD_REQUEST,
            Json(Response::Error(format!("RegexError: {:?}", e).to_string())),
        )
    }
}
