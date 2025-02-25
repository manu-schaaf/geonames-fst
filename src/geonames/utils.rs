use std::collections::HashSet;
use std::{collections::HashMap, fs::File};
use std::{f32, io};

use anyhow::anyhow;

use super::data::{GeoNamesEntry, MatchType};

pub(crate) fn parse_geonames_file(
    path: &str,
    query_pairs: &mut Vec<(String, MatchType)>,
    geonames: &mut HashMap<u64, GeoNamesEntry>,
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
            query_pairs.push((name_ascii, MatchType::AsciiName { id }));
        }
        // set_of_seen_names.insert(name.clone());
        query_pairs.push((name.clone(), MatchType::Name { id }));

        geonames.insert(
            id,
            GeoNamesEntry {
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

pub(crate) fn parse_alternate_names_file(
    path: &str,
    query_pairs: &mut Vec<(String, MatchType)>,
    geonames: &HashMap<u64, GeoNamesEntry>,
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

        if !geonames.contains_key(&id) {
            continue;
        }

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
                query_pairs.push((name, MatchType::PreferredName { id, lang }));
            }
            (false, true, false, false) => {
                query_pairs.push((name, MatchType::ShortName { id, lang }));
            }
            (false, false, true, false) => {
                query_pairs.push((name, MatchType::Colloquial { id, lang }));
            }
            (false, false, false, true) => {
                query_pairs.push((name, MatchType::Historic { id, lang, from, to }));
            }
            _ => {
                query_pairs.push((name, MatchType::Alternate { id, lang }));
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
