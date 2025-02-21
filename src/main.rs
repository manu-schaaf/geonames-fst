use std::{f32, io};
use std::{collections::HashMap, fs::File};

use anyhow::anyhow;
use fst::{automaton::Levenshtein, IntoStreamer, Map, MapBuilder, Streamer};
use levenshtein::levenshtein;
use memmap::Mmap;

fn main() -> anyhow::Result<()> {
    let mut search_terms: Vec<(String, u64)> = Vec::new();
    let mut geoname_data: HashMap<u64, (f32, f32, String, String)> = HashMap::new();

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(io::BufReader::new(File::open(
            "data/geonames/cities500.txt",
            // "data/geonames/head_cities500.txt",
        )?));

    for row in rdr.records() {
        let record = row?;
        let geoname_id: u64 = record.get(0).ok_or(anyhow!("no geoname_id"))?.parse()?;
        search_terms.push((
            record.get(1).ok_or(anyhow!("no name"))?.to_string(),
            geoname_id,
        ));
        if let Some(alternate_names) = record.get(3) {
            for name in alternate_names.split(',') {
                if name.trim().is_empty() {
                    continue;
                }
                search_terms.push((name.trim().to_string(), geoname_id));
            }
        }
        geoname_data.insert(
            geoname_id,
            (
                record.get(4).map(str::parse).map_or(f32::NAN, |v| v.unwrap_or(f32::NAN)),
                record.get(5).map(str::parse).map_or(f32::NAN, |v| v.unwrap_or(f32::NAN)),
                record
                    .get(7)
                    .unwrap_or("<missing:feature_code>")
                    .to_string(),
                record
                    .get(8)
                    .unwrap_or("<missing:country_code>")
                    .to_string(),
            ),
        );
    }

    println!("Read {} search terms", search_terms.len());

    search_terms.sort();
    // We currently only handle unique, unambigous identifiers
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
            let val = geoname_data.get(&val).ok_or(anyhow!("no geoname data"))?;
            let dist = levenshtein(&buffer.trim(), &key);
            println!("key: {key:?}, val: {val:?}, size: {size:?}, dist: {dist:?}");
        }
        println!();
        buffer.clear();
    }

    Ok(())
}
