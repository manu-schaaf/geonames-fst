use std::collections::HashMap;

use fst::{Automaton, IntoStreamer, Map, MapBuilder, Streamer};
use levenshtein::levenshtein as levenshtein_dist;

use super::data::{GeoNamesData, GeoNamesSearchResult, GeoNamesSearchResultWithDist, MatchType};
use super::utils::{parse_alternate_names_file, parse_geonames_file};

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
        results.sort();

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
        results.sort();

        results
    }

    pub fn build(
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
                parse_alternate_names_file(
                    &path,
                    &mut query_pairs,
                    gn_alternate_languages.as_ref(),
                )?;
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
}
