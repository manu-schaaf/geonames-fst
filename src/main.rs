use std::{collections::HashMap, fs::File};
use std::{f32, io};

use anyhow::anyhow;
use fst::{automaton::Levenshtein, IntoStreamer, Map, MapBuilder, Streamer};
use levenshtein::levenshtein;
use memmap::Mmap;
use rocksdb::DB;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GeoNamesData {
    name: String,
    latitude: f32,
    longitude: f32,
    feature_code: String,
    country_code: String,
}

fn main() -> anyhow::Result<()> {
    let mut search_terms: Vec<(String, u64)> = Vec::new();
    let db = DB::open_default("_rocksdb").unwrap();

    parse_geonames_file("data/geonames/cities500.txt", &mut search_terms, &db)?;
    parse_geonames_file("data/geonames/DE.txt", &mut search_terms, &db)?;

    println!("Read {} search terms", search_terms.len());

    search_terms.sort();
    // We currently only handle unique, unambiguous identifiers
    search_terms.dedup_by(|(a, _), (b, _)| a == b);

    println!(
        "Sorted and deduplicated to {} search terms",
        search_terms.len()
    );

    // This is where we'll write our map to.
    let mut wtr = io::BufWriter::new(File::create("map.fst")?);

    // Create a builder that can be used to insert new key-value pairs.
    let mut build = MapBuilder::new(wtr)?;
    for (term, geoname_id) in search_terms {
        build.insert(term, geoname_id)?;
    }

    // Finish construction of the map and flush its contents to disk.
    build.finish()?;

    println!("Wrote map to disk");

    let mmap = unsafe { Mmap::map(&File::open("map.fst")?)? };
    let map = Map::new(mmap)?;

    println!("Read map from disk");

    let mut buffer = String::new();
    loop {
        io::stdin().read_line(&mut buffer)?;

        let max_distance: u32 = (buffer.trim().len() / 8).try_into().unwrap_or(4u32);
        let query = Levenshtein::new(&buffer.trim(), max_distance)
            .or_else(|_| Levenshtein::new(&buffer, 1))?;
        let mut stream = map.search_with_state(&query).into_stream();

        while let Some((key, val, size)) = stream.next() {
            let key = String::from_utf8_lossy(key).to_string();

            let val = db.get(val.to_be_bytes())?.unwrap();
            let val: GeoNamesData = bincode::deserialize(&val).unwrap();

            let dist = levenshtein(&buffer.trim(), &key);
            println!("key: {key:?}, size: {size:?}, dist: {dist:?}, val: {val:?}");
        }
        println!();
        buffer.clear();
    }

    Ok(())
}

fn parse_geonames_file(
    path: &str,
    search_terms: &mut Vec<(String, u64)>,
    db: &DB,
) -> Result<(), anyhow::Error> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(io::BufReader::new(File::open(path)?));
    for row in rdr.records() {
        let record = row?;

        let geoname_id: u64 = record.get(0).ok_or(anyhow!("no geoname_id"))?.parse()?;
        let name: &str = record.get(1).ok_or(anyhow!("no name"))?;

        let latitude: f32 = parse_float_else_nan(record.get(4));
        let longitude: f32 = parse_float_else_nan(record.get(5));
        let feature_code: &str = record.get(7).unwrap_or("<missing:feature_code>");
        let country_code: &str = record.get(8).unwrap_or("<missing:country_code>");

        let data = GeoNamesData {
            name: name.to_string(),
            latitude,
            longitude,
            feature_code: feature_code.to_string(),
            country_code: country_code.to_string(),
        };

        db.put(geoname_id.to_be_bytes(), bincode::serialize(&data)?)
            .unwrap();

        search_terms.push((name.to_string(), geoname_id));
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
