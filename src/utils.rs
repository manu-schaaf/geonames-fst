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
pub enum MatchType {
    Literal(String),
    Alternative(String),
    Historic(String),
    ASCII(String),
}

#[derive(Debug, Serialize)]
pub struct GeoNamesSearchResult {
    key: String,
    name: String,
    latitude: f32,
    longitude: f32,
    feature_class: String,
    feature_code: String,
    country_code: String,
}

#[derive(Debug, Serialize)]
pub struct GeoNamesSearchResultWithDist {
    key: String,
    name: String,
    latitude: f32,
    longitude: f32,
    feature_class: String,
    feature_code: String,
    country_code: String,
    distance: usize,
}

impl GeoNamesSearchResult {
    pub fn new(key: &str, gnd: &GeoNamesData) -> Self {
        GeoNamesSearchResult {
            key: key.to_string(),
            name: gnd.name.clone(),
            latitude: gnd.latitude,
            longitude: gnd.longitude,
            feature_class: gnd.feature_class.clone(),
            feature_code: gnd.feature_code.clone(),
            country_code: gnd.country_code.clone(),
        }
    }
}

impl GeoNamesSearchResultWithDist {
    pub fn new(key: &str, gnd: &GeoNamesData, dist: usize) -> Self {
        GeoNamesSearchResultWithDist {
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

pub struct GeoNamesSearcher {
    pub map: Map<Vec<u8>>,
    pub data_store: HashMap<u64, GeoNamesData>,
}

impl GeoNamesSearcher {
    pub fn get(&self, query: &str) -> Vec<GeoNamesSearchResult> {
        self.map
            .get(query)
            .map(|gnd| {
                let gnd: &GeoNamesData = self.data_store.get(&gnd).unwrap();
                vec![GeoNamesSearchResult::new(query, gnd)]
            })
            .unwrap_or_default()
    }

    pub fn search(&self, query: impl Automaton) -> Vec<GeoNamesSearchResult> {
        let mut stream = self.map.search(&query).into_stream();
        let mut results = Vec::new();
        while let Some((key, gnd)) = stream.next() {
            let key = String::from_utf8_lossy(key).to_string();
            let gnd: &GeoNamesData = self.data_store.get(&gnd).unwrap();
            results.push(GeoNamesSearchResult::new(&key, gnd));
        }

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

            let gnd: &GeoNamesData = self.data_store.get(&gnd).unwrap();
            results.push(GeoNamesSearchResultWithDist::new(&key, gnd, dist));
        }
        results.sort_by(|a, b| a.distance.cmp(&b.distance));

        results
    }
}

pub(crate) fn build_fst() -> Result<GeoNamesSearcher, anyhow::Error> {
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
    Ok(GeoNamesSearcher { map, data_store })
}

pub(crate) fn parse_geonames_file(
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
            id: geoname_id,
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

pub(crate) fn parse_float_else_nan(maybe_str: Option<&str>) -> f32 {
    if let Some(maybe_str) = maybe_str {
        maybe_str.trim().parse::<f32>().unwrap_or(f32::NAN)
    } else {
        f32::NAN
    }
}
