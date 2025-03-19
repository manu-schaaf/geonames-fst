use aide::axum::IntoApiResponse;
use axum::extract::State;
use axum::{http::StatusCode, Json};
use schemars::JsonSchema;
use serde::Serialize;

use crate::routes::FilterResults;
use crate::AppState;

#[derive(Serialize, JsonSchema)]
pub(crate) struct Meta {
    number_of_geonames: usize,
    fst_size: usize,
}

#[derive(Serialize, JsonSchema)]
pub(crate) struct Capability {
    supported_languages: Option<Vec<String>>,
    reproducible: bool,
}

#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub(crate) enum Param<T: Serialize> {
    Type {
        r#type: String,
        desc: String,
    },
    Choices {
        r#type: String,
        desc: String,
        choices: Vec<T>,
    },
}

impl<T: Serialize> Param<T> {
    fn typ(r#type: &str, desc: &str) -> Self {
        Param::Type {
            r#type: r#type.to_string(),
            desc: desc.to_string(),
        }
    }
    fn choices(r#type: &str, desc: &str, choices: Vec<T>) -> Self {
        Param::Choices {
            r#type: r#type.to_string(),
            desc: desc.to_string(),
            choices,
        }
    }
}

#[derive(Serialize, JsonSchema)]
pub(crate) struct Parameters {
    annotation_type: Param<&'static str>,
    return_type: Param<&'static str>,
    mode: Param<&'static str>,
    max_dist: Param<u32>,
    state_limit: Param<u32>,
    filter: Param<FilterResults>,
}

#[derive(Serialize, JsonSchema)]
pub(crate) struct Documentation {
    annotator_name: &'static str,
    version: &'static str,
    implementation_lang: Option<&'static str>,
    meta: Option<Meta>,
    // docker_container_id: Option<String>,
    parameters: Parameters,
    capability: Capability,
    // implementation_specific: Option<String>,
}

pub(crate) async fn v1_documentation(State(state): State<AppState>) -> impl IntoApiResponse {
    (
        StatusCode::OK,
        Json(Documentation {
            annotator_name: "DUUI GeoNames FST",
            version: env!("CARGO_PKG_VERSION"),
            implementation_lang: Some("Rust"),
            meta: Some(Meta {
                number_of_geonames: state.searcher.geonames.len(),
                fst_size: state.searcher.map.len(),
            }),
            // docker_container_id: Some("".to_string()),
            parameters: Parameters {
                annotation_type: Param::typ("String", "The annotation type to extract from the source document as a fully qualified class name."),
                return_type: Param::choices("String", "The return type: either one or all matching GeoNames.", vec!["first", "all"]),
                mode: Param::choices(
                    "String",
                    "The search mode to use.",
                    vec![
                        "find",
                        "starts_with",
                        "fuzzy",
                        "levenshtein",
                    ],
                ),
                max_dist: Param::typ("int", "Positive number of maximum Levenshtein distance between the input string and the search results."),
                state_limit: Param::typ("int", "Positive number that represents the maximum number of states in the finite state transducer."),
                filter: Param::typ(
                    "dict",
                    "An optional dictionary of (each optional) feature_class (a GeoNames feature class, e.g. 'P' for populated place), feature_code (a GeoNames feature code, e.g. 'MT' for mountains), and country_code (a GeoNames country code, e.g. 'DE' for Germany)."
                )
            },
            capability: Capability { supported_languages: state.languages, reproducible: true },
            // implementation_specific: todo!(),
        }),
    )
}
