use std::time::{self, UNIX_EPOCH};

use aide::axum::IntoApiResponse;
use aide::transform::TransformOperation;
use axum::extract::State;
use axum::{http::StatusCode, Json};
use fst::automaton::{Str, Subsequence};
use fst::Automaton;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_aux::prelude::*;

use crate::geonames::data::GeoNamesSearchResultWithDist;
use crate::geonames::searcher::GeoNamesSearcher;
use crate::routes::docs::DocResults;
use crate::routes::filter_results;
use crate::routes::find::RequestOptsFind;
use crate::routes::fuzzy::RequestOptsFuzzy;
use crate::routes::levenshtein::{levenshtein_inner, RequestOptsLevenshtein};
use crate::routes::starts_with::RequestOptsStartsWith;
use crate::AppState;

fn _default_entity() -> Entity {
    Entity {
        reference: 0,
        text: "Großer Feldberg".to_string(),
    }
}

fn _lua_number_to_int(lua_number: f64) -> u32 {
    lua_number as u32
}

#[derive(Deserialize, JsonSchema)]
#[schemars(default = "_default_entity")]
pub(crate) struct Entity {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub reference: u32,
    pub text: String,
}

#[derive(serde::Serialize, schemars::JsonSchema)]
pub(crate) struct AnnotatedEntity {
    pub reference: u32,
    #[serde(flatten)]
    pub annotation: GeoNamesSearchResultWithDist,
}

impl AnnotatedEntity {
    pub fn annotate(entity: &Entity, annotation: GeoNamesSearchResultWithDist) -> Self {
        Self {
            reference: entity.reference,
            annotation,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub(crate) enum SearchMode {
    Find(RequestOptsFind),
    // Regex(RequestOptsRegex),
    StartsWith(RequestOptsStartsWith),
    Fuzzy(RequestOptsFuzzy),
    Levenshtein(RequestOptsLevenshtein),
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ResultSelection {
    First,
    All,
}

impl Default for ResultSelection {
    fn default() -> Self {
        Self::First
    }
}

impl ResultSelection {
    pub fn apply<T: Into<GeoNamesSearchResultWithDist>>(
        &self,
        entity: &Entity,
        items: Vec<T>,
    ) -> Option<Vec<AnnotatedEntity>> {
        match self {
            Self::First => items
                .into_iter()
                .next()
                .map(|annotation| vec![AnnotatedEntity::annotate(entity, annotation.into())]),
            Self::All => items
                .into_iter()
                .map(|annotation| Some(AnnotatedEntity::annotate(entity, annotation.into())))
                .collect(),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct RequestProcess {
    pub queries: Vec<Entity>,
    #[schemars(default = "ResultSelection::default")]
    pub result_selection: ResultSelection,
    #[serde(flatten)]
    pub options: SearchMode,
}

#[derive(serde::Serialize, schemars::JsonSchema)]
pub(crate) struct DocumentModification {
    pub user: String,
    pub timestamp: u64,
    pub comment: String,
}

impl Default for DocumentModification {
    fn default() -> Self {
        Self {
            user: env!("CARGO_PKG_NAME").to_string(),
            timestamp: time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            comment: "".to_string(),
        }
    }
}

impl DocumentModification {
    fn with_comment(comment: String) -> Self {
        Self {
            comment,
            ..Default::default()
        }
    }

    fn with_duui_commment(state: &AppState) -> Self {
        let mut comment = Vec::new();
        if let Some(timestamp) = state.timestamp.as_ref() {
            comment.push(format!("GeoNames Date: {timestamp}"));
        }
        if let Some(languages) = state.languages.as_ref() {
            comment.push(format!(
                "Languages: {}",
                languages
                    .iter()
                    .map(|l| format!("'{l}'"))
                    .collect::<Vec<String>>()
                    .join(", ")
            ));
        }
        Self::with_comment(comment.join("; "))
    }
}

#[derive(serde::Serialize, schemars::JsonSchema)]
pub(crate) struct Results {
    pub results: Vec<AnnotatedEntity>,
    pub modification: DocumentModification,
}

pub(crate) async fn v1_process(
    State(state): State<AppState>,
    Json(request): Json<RequestProcess>,
) -> impl IntoApiResponse {
    let modification = DocumentModification::with_duui_commment(&state);

    let results = match request.options {
        SearchMode::Find(options) => process_find(
            &state.searcher,
            request.queries,
            options,
            request.result_selection,
        ),
        // SearchMode::Regex(options) => todo!(),
        SearchMode::StartsWith(options) => process_starts_with(
            &state.searcher,
            request.queries,
            options,
            request.result_selection,
        ),
        SearchMode::Fuzzy(options) => process_fuzzy(
            &state.searcher,
            request.queries,
            options,
            request.result_selection,
        ),
        SearchMode::Levenshtein(options) => process_levenshtein(
            &state.searcher,
            request.queries,
            options,
            request.result_selection,
        ),
    };
    (
        StatusCode::OK,
        Json(Results {
            results,
            modification,
        }),
    )
}

fn process_find(
    searcher: &GeoNamesSearcher,
    queries: Vec<Entity>,
    options: RequestOptsFind,
    return_type: ResultSelection,
) -> Vec<AnnotatedEntity> {
    queries
        .iter()
        .filter_map(|entity| {
            return_type.apply(
                entity,
                filter_results(searcher.find(&entity.text), &options.filter),
            )
        })
        .flatten()
        .collect()
}

fn process_starts_with(
    searcher: &GeoNamesSearcher,
    queries: Vec<Entity>,
    options: RequestOptsStartsWith,
    return_type: ResultSelection,
) -> Vec<AnnotatedEntity> {
    queries
        .iter()
        .filter_map(|entity| {
            let query = Str::new(&entity.text).starts_with();
            let results = searcher.search_with_dist(query, &entity.text, Some(options.max_dist));
            let results = filter_results(results, &options.filter);
            return_type.apply(entity, results)
        })
        .flatten()
        .collect()
}

fn process_fuzzy(
    searcher: &GeoNamesSearcher,
    queries: Vec<Entity>,
    options: RequestOptsFuzzy,
    return_type: ResultSelection,
) -> Vec<AnnotatedEntity> {
    queries
        .iter()
        .filter_map(|entity| {
            let query = Subsequence::new(&entity.text);
            let results = searcher.search_with_dist(query, &entity.text, Some(options.max_dist));
            let results = filter_results(results, &options.filter);
            return_type.apply(entity, results)
        })
        .flatten()
        .collect()
}

fn process_levenshtein(
    searcher: &GeoNamesSearcher,
    queries: Vec<Entity>,
    options: RequestOptsLevenshtein,
    return_type: ResultSelection,
) -> Vec<AnnotatedEntity> {
    queries
        .iter()
        .filter_map(|entity| {
            levenshtein_inner(
                searcher,
                &entity.text,
                options.state_limit,
                options.max_dist,
                &options.filter,
            )
            .ok()
            .and_then(|results| return_type.apply(entity, results))
        })
        .flatten()
        .collect()
}

pub(crate) fn v1_process_docs(op: TransformOperation) -> TransformOperation {
    op.description("Tag GeoNames in a list of entities given as offsets and covered text.")
        .response::<200, Json<DocResults<Vec<GeoNamesSearchResultWithDist>>>>()
}
