pub mod docs;
pub mod find;
pub mod fuzzy;
pub mod levenshtein;
pub mod regex;
pub mod regex_automaton;
pub mod starts_with;

use find::{find, find_docs};
use fuzzy::{fuzzy, fuzzy_docs};
use levenshtein::{levenshtein, levenshtein_docs};
use regex::{regex, regex_docs};
use starts_with::{starts_with, starts_with_docs};

use crate::geonames::data;

use aide::axum::{routing::post_with, ApiRouter};

use crate::AppState;

pub(crate) fn geonames_routes(state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route("/find", post_with(find, find_docs))
        .api_route("/regex", post_with(regex, regex_docs))
        .api_route("/starts_with", post_with(starts_with, starts_with_docs))
        .api_route("/fuzzy", post_with(fuzzy, fuzzy_docs))
        .api_route("/levenshtein", post_with(levenshtein, levenshtein_docs))
        .with_state(state)
}

#[derive(serde::Serialize, schemars::JsonSchema)]
pub(crate) enum Response {
    #[serde(rename = "results")]
    Results(Vec<data::GeoNamesSearchResult>),
    #[serde(rename = "results")]
    ResultsWithDist(Vec<data::GeoNamesSearchResultWithDist>),
    #[serde(rename = "error")]
    Error(String),
}

fn _default_string_none() -> Option<String> {
    None
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct FilterResults {
    #[schemars(default = "_default_string_none")]
    pub feature_class: Option<String>,
    #[schemars(default = "_default_string_none")]
    pub feature_code: Option<String>,
    #[schemars(default = "_default_string_none")]
    pub country_code: Option<String>,
}

pub(crate) fn _schemars_default_filter() -> Option<FilterResults> {
    None
}

pub(crate) fn filter_results<T>(mut results: Vec<T>, filter: &Option<FilterResults>) -> Vec<T>
where
    T: data::Entry,
{
    if let Some(filter) = filter {
        if let Some(feature_class) = &filter.feature_class {
            results.retain(|r| r.entry().feature_class.eq(feature_class));
        }
        if let Some(feature_code) = &filter.feature_code {
            results.retain(|r| r.entry().feature_code.eq(feature_code));
        }
        if let Some(country_code) = &filter.country_code {
            results.retain(|r| r.entry().country_code.eq(country_code));
        }
    }
    results
}
