use std::{collections::HashMap, fs::File};
use std::{f32, io};

use anyhow::anyhow;
use fst::{automaton::Levenshtein, IntoStreamer, Map, MapBuilder, Streamer};
use levenshtein::levenshtein;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GeoNamesData {
    name: String,
    latitude: f32,
    longitude: f32,
    feature_class: String,
    feature_code: String,
    country_code: String,
}

fn main() -> anyhow::Result<()> {
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
    // We currently only handle unique, unambiguous identifiers
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

    let mut buffer = String::new();
    loop {
        io::stdin().read_line(&mut buffer)?;

        let max_distance: u32 = (buffer.trim().len() / 4).try_into().unwrap_or(4u32);
        let query = Levenshtein::new(&buffer.trim(), max_distance)
            .or_else(|_| Levenshtein::new(&buffer, 1))?;
        let mut stream = map.search_with_state(&query).into_stream();

        let mut results = Vec::new();
        while let Some((key, val, _)) = stream.next() {
            let key = String::from_utf8_lossy(key).to_string();

            let dist = levenshtein(&buffer.trim(), &key);

            let val = data_store.get(&val).unwrap();

            results.push((key, dist, val));
        }

        // sort by dist
        results.sort_by(|a, b| a.1.cmp(&b.1));
        for result in results {
            println!("{} dist:{} {:?}", result.0, result.1, result.2);
        }
        println!();
        buffer.clear();
    }

    Ok(())
}

fn parse_geonames_file(
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

fn parse_float_else_nan(maybe_str: Option<&str>) -> f32 {
    if let Some(maybe_str) = maybe_str {
        maybe_str.trim().parse::<f32>().unwrap_or(f32::NAN)
    } else {
        f32::NAN
    }
}
