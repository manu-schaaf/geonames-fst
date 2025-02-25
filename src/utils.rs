use std::collections::HashSet;
use std::{collections::HashMap, fs::File};
use std::{f32, io};

use anyhow::anyhow;
use fst::{Automaton, IntoStreamer, Map, MapBuilder, Streamer};
use levenshtein::levenshtein as levenshtein_dist;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct GeoNamesData {
    pub id: u64,
    pub name: String,
    pub latitude: f32,
    pub longitude: f32,
    pub feature_class: String,
    pub feature_code: String,
    pub country_code: String,
}

#[derive(Debug, Serialize)]
pub struct GeoNamesSearchResult {
    key: MatchKey,
    name: String,
    latitude: f32,
    longitude: f32,
    feature_class: String,
    feature_code: String,
    country_code: String,
}

#[derive(Debug, Serialize)]
pub struct GeoNamesSearchResultWithDist {
    key: MatchKey,
    name: String,
    latitude: f32,
    longitude: f32,
    feature_class: String,
    feature_code: String,
    country_code: String,
    distance: usize,
}

impl GeoNamesSearchResult {
    pub fn new(key: &str, mtch: &MatchType, gnd: &GeoNamesData) -> Self {
        GeoNamesSearchResult {
            name: gnd.name.clone(),
            latitude: gnd.latitude,
            longitude: gnd.longitude,
            feature_class: gnd.feature_class.clone(),
            feature_code: gnd.feature_code.clone(),
            country_code: gnd.country_code.clone(),
            key: MatchKey {
                name: key.to_string(),
                typ: mtch.clone(),
            },
        }
    }
}

impl GeoNamesSearchResultWithDist {
    pub fn new(key: &str, mtch: &MatchType, gnd: &GeoNamesData, dist: usize) -> Self {
        GeoNamesSearchResultWithDist {
            name: gnd.name.clone(),
            latitude: gnd.latitude,
            longitude: gnd.longitude,
            feature_class: gnd.feature_class.clone(),
            feature_code: gnd.feature_code.clone(),
            country_code: gnd.country_code.clone(),
            distance: dist,
            key: MatchKey {
                name: key.to_string(),
                typ: mtch.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum MatchType {
    Name {
        id: u64,
    },
    AsciiName {
        id: u64,
    },
    PreferredName {
        id: u64,
        lang: String,
    },
    ShortName {
        id: u64,
        lang: String,
    },
    Colloquial {
        id: u64,
        lang: String,
    },
    Historic {
        id: u64,
        lang: String,
        from: String,
        to: String,
    },
    Alternate {
        id: u64,
        lang: String,
    },
}

impl MatchType {
    fn id(&self) -> u64 {
        match self {
            MatchType::Name { id } => *id,
            MatchType::AsciiName { id } => *id,
            MatchType::PreferredName { id, .. } => *id,
            MatchType::ShortName { id, .. } => *id,
            MatchType::Colloquial { id, .. } => *id,
            MatchType::Historic { id, .. } => *id,
            MatchType::Alternate { id, .. } => *id,
        }
    }

    fn ord(&self) -> u8 {
        match self {
            MatchType::Name { .. } => 0,
            MatchType::AsciiName { .. } => 1,
            MatchType::PreferredName { .. } => 2,
            MatchType::ShortName { .. } => 3,
            MatchType::Colloquial { .. } => 4,
            MatchType::Historic { .. } => 5,
            MatchType::Alternate { .. } => 6,
        }
    }
}

impl Ord for MatchType {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.id() == other.id() {
            self.ord().cmp(&other.ord())
        } else {
            self.id().cmp(&other.id())
        }
    }
}

impl PartialOrd for MatchType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Serialize)]
struct MatchKey {
    name: String,
    #[serde(flatten)]
    typ: MatchType,
}

pub struct GeoNamesSearcher {
    pub map: Map<Vec<u8>>,
    pub geonames: HashMap<u64, GeoNamesData>,
    search_matches: Vec<Vec<MatchType>>,
}

impl GeoNamesSearcher {
    pub fn get(&self, query: &str) -> Vec<GeoNamesSearchResult> {
        self.map
            .get(query)
            .map(|gnd| {
                let matches = &self.search_matches[gnd as usize];
                matches
                    .iter()
                    .map(|mtch| {
                        let gnd = self.geonames.get(&mtch.id()).unwrap();
                        GeoNamesSearchResult::new(query, mtch, gnd)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn search(&self, query: impl Automaton) -> Vec<GeoNamesSearchResult> {
        let mut stream = self.map.search(&query).into_stream();

        let mut results = Vec::new();
        while let Some((key, gnd)) = stream.next() {
            let key = String::from_utf8_lossy(key).to_string();
            let matches = &self.search_matches[gnd as usize];
            results.extend(matches.iter().map(|mtch| {
                let gnd = self.geonames.get(&mtch.id()).unwrap();
                GeoNamesSearchResult::new(&key, mtch, gnd)
            }));
        }
        results.sort_by(|a, b| a.key.typ.cmp(&b.key.typ));

        results
    }

    pub fn search_with_dist(
        &self,
        query: impl Automaton,
        raw: &str,
        max_dist: &Option<u32>,
    ) -> Vec<GeoNamesSearchResultWithDist> {
        let mut stream = self.map.search(&query).into_stream();
        let mut results = Vec::new();
        while let Some((key, gnd)) = stream.next() {
            let key = String::from_utf8_lossy(key).to_string();
            let dist = levenshtein_dist(raw, &key);
            if let Some(distance) = max_dist {
                if dist > (*distance as usize) {
                    continue;
                }
            }
            let matches = &self.search_matches[gnd as usize];
            for mtch in matches {
                let gnd: &GeoNamesData = self.geonames.get(&mtch.id()).unwrap();
                results.push(GeoNamesSearchResultWithDist::new(&key, mtch, gnd, dist));
            }
        }
        results.sort_by(|a, b| a.distance.cmp(&b.distance));

        results
    }
}

pub(crate) fn build_searcher(
    gn_paths: Vec<String>,
    gn_alternate_paths: Option<Vec<String>>,
    gn_alternate_languages: Option<Vec<String>>,
) -> Result<GeoNamesSearcher, anyhow::Error> {
    let mut query_pairs: Vec<(String, MatchType)> = Vec::new();
    let mut geonames: HashMap<u64, GeoNamesData> = HashMap::new();
    for path in gn_paths {
        parse_geonames_file(&path, &mut query_pairs, &mut geonames)?;
    }
    println!("Read {} search terms", query_pairs.len());

    if let Some(gn_alternate_paths) = gn_alternate_paths {
        for path in gn_alternate_paths {
            parse_alternate_names_file(&path, &mut query_pairs, gn_alternate_languages.as_ref())?;
        }
        println!(
            "Read {} search terms (including alternate names)",
            query_pairs.len()
        );
    }

    query_pairs.sort_by(|a, b| a.0.cmp(&b.0));

    let mut last_term: String = "".to_string();
    let mut search_terms: Vec<String> = Vec::new();
    let mut search_matches: Vec<Vec<MatchType>> = Vec::new();

    for (term, mtch) in query_pairs.into_iter() {
        if term.is_empty() {
            continue;
        }

        if term == last_term {
            search_matches.last_mut().unwrap().push(mtch);
        } else {
            search_terms.push(term.clone());
            search_matches.push(vec![mtch]);
        }
        last_term = term;
    }

    let mut build = MapBuilder::memory();
    search_terms.iter().enumerate().for_each(|(i, term)| {
        build.insert(term, i as u64).unwrap();
    });

    let bytes = build.into_inner()?;
    let num_bytes = bytes.len();
    let map = Map::new(bytes)?;
    println!("Built FST with {} bytes", num_bytes);

    Ok(GeoNamesSearcher {
        map,
        geonames,
        search_matches,
    })
}

pub(crate) fn parse_geonames_file(
    path: &str,
    search_terms: &mut Vec<(String, MatchType)>,
    data_store: &mut HashMap<u64, GeoNamesData>,
) -> Result<(), anyhow::Error> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(io::BufReader::new(File::open(path)?));

    for row in rdr.records() {
        let record = row?;

        let id: u64 = record.get(0).ok_or(anyhow!("no geoname_id"))?.parse()?;
        let name: String = record.get(1).ok_or(anyhow!("no name"))?.to_string();
        let name_ascii: String = record.get(2).ok_or(anyhow!("no ascii name"))?.to_string();

        let latitude: f32 = parse_float_else_nan(record.get(4));
        let longitude: f32 = parse_float_else_nan(record.get(5));
        let feature_class: String = record.get(6).unwrap_or("<missing>").to_string();
        let feature_code: String = record.get(7).unwrap_or("<missing>").to_string();
        let country_code: String = record.get(8).unwrap_or("<missing>").to_string();

        if name_ascii != name {
            // set_of_seen_names.insert(name_ascii.clone());
            search_terms.push((name_ascii, MatchType::AsciiName { id }));
        }
        // set_of_seen_names.insert(name.clone());
        search_terms.push((name.clone(), MatchType::Name { id }));

        data_store.insert(
            id,
            GeoNamesData {
                id,
                name,
                latitude,
                longitude,
                feature_class,
                feature_code,
                country_code,
            },
        );
    }
    Ok(())
}

fn parse_alternate_names_file(
    path: &str,
    search_terms: &mut Vec<(String, MatchType)>,
    include_languages: Option<&Vec<String>>,
) -> Result<(), anyhow::Error> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(io::BufReader::new(File::open(path)?));

    let include_languages: Option<HashSet<&String>> = include_languages.map(HashSet::from_iter);

    for row in rdr.records() {
        let record = row?;

        let lang: String = record.get(2).ok_or(anyhow!("no language"))?.to_string();
        if include_languages
            .as_ref()
            .is_some_and(|set| !set.contains(&lang))
        {
            continue;
        }

        let id: u64 = record.get(1).ok_or(anyhow!("no geoname_id"))?.parse()?;
        let lang = lang.to_string();
        let name: String = record.get(3).ok_or(anyhow!("no name"))?.to_string();

        let preferred: bool = record.get(4).ok_or(anyhow!("no preferred"))?.eq("1");
        let short: bool = record.get(5).ok_or(anyhow!("no short"))?.eq("1");
        let colloquial: bool = record.get(6).ok_or(anyhow!("no colloquial"))?.eq("1");
        let historic: bool = record.get(7).ok_or(anyhow!("no historic"))?.eq("1");
        let from: String = record.get(8).unwrap_or("").to_string();
        let to: String = record.get(9).unwrap_or("").to_string();

        match (preferred, short, colloquial, historic) {
            (true, false, false, false) => {
                search_terms.push((name, MatchType::PreferredName { id, lang }));
            }
            (false, true, false, false) => {
                search_terms.push((name, MatchType::ShortName { id, lang }));
            }
            (false, false, true, false) => {
                search_terms.push((name, MatchType::Colloquial { id, lang }));
            }
            (false, false, false, true) => {
                search_terms.push((name, MatchType::Historic { id, lang, from, to }));
            }
            _ => {
                search_terms.push((name, MatchType::Alternate { id, lang }));
            }
        }
    }
    Ok(())
}

pub(crate) fn parse_float_else_nan(maybe_str: Option<&str>) -> f32 {
    if let Some(maybe_str) = maybe_str {
        maybe_str.trim().parse::<f32>().unwrap_or(f32::NAN)
    } else {
        f32::NAN
    }
}
