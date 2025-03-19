use std::str::FromStr;

use aide::axum::IntoApiResponse;
use aide::transform::TransformOperation;
use axum::extract::State;
use axum::{http::StatusCode, Json};
use schemars::JsonSchema;
use serde::Deserialize;

use super::docs::{DocError, DocResults};
use super::regex_automaton::RegexSearchAutomaton;
use super::{filter_results, FilterResults, Response, _schemars_default_filter};
use crate::geonames::data::GeoNamesSearchResult;
use crate::AppState;

#[derive(Deserialize, JsonSchema)]
pub(crate) struct RequestOptsRegex {
    #[schemars(
        default = "_schemars_default_filter",
        skip_serializing_if = "Option::is_none"
    )]
    pub filter: Option<FilterResults>,
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

    #[serde(flatten)]
    pub opts: RequestOptsRegex,
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
        let results = filter_results(state.searcher.search(query), &request.opts.filter);

        (StatusCode::OK, Json(Response::Results(results)))
    } else {
        let e = dfa.unwrap_err();

        (
            StatusCode::BAD_REQUEST,
            Json(Response::Error(format!("RegexError: {:?}", e).to_string())),
        )
    }
}

pub(crate) fn regex_docs(op: TransformOperation) -> TransformOperation {
    op.description("Find all GeoNames entries with the specified regex.")
        .response::<200, Json<DocResults<GeoNamesSearchResult>>>()
        .response_with::<400, Json<DocError>, _>(|t| t.description("The query was empty."))
}
