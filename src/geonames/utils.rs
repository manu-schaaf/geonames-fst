use std::collections::HashSet;
use std::io::{BufReader, Read};
use std::path::Path;
use std::{collections::HashMap, fs::File};
use std::f32;

use anyhow::anyhow;
use tracing::{event, Level};

#[cfg(feature = "bzip2")]
use bzip2_rs::DecoderReader as Bzip2Decoder;
#[cfg(feature = "gzip")]
use flate2::bufread::GzDecoder;
#[cfg(feature = "xz")]
use xz::bufread::XzDecoder;

use super::data::{GeoNamesEntry, MatchType};

pub fn get_reader(path: &Path) -> anyhow::Result<Box<dyn Read>> {
    let file = File::open(path).expect("Could not open file");
    let buf_reader: BufReader<File> = BufReader::new(file);

    let extension = match Path::new(path).extension() {
        None => "<none>",
        Some(ext) => ext.to_str().unwrap(),
    };
    match extension {
        // No compression, the only path that is always supported
        "txt" | "<none>" => Ok(Box::new(buf_reader)),

        // GeoNames dumps come in zip files, which in turn may contain multiple files.
        // We require the user to unpack the zip file first, passing only the required files into the program.
        "zip" => Err(anyhow!("Unpacked GeoNames dump files are not supported! Please unpack the GeoNames zip file first.")),

        #[cfg(feature = "bzip2")]
        "bz2" => Ok(Box::new(Bzip2Decoder::new(buf_reader))),
        #[cfg(not(feature = "bzip2"))]
        "bz2" => Err(anyhow!("This binary was not compiled with the bzip2 feature enabled! Cannot read {path:?}.")),
        
        #[cfg(feature = "gzip")]
        "gz" => Ok(Box::new(GzDecoder::new(buf_reader))),
        #[cfg(not(feature = "gzip"))]
        "gz" => Err(anyhow!("This binary was not compiled with the gzip feature enabled! Cannot read {path:?}.")),

        #[cfg(feature = "xz")]
        "xz" => Ok(Box::new(XzDecoder::new(buf_reader))),
        #[cfg(not(feature = "xz"))]
        "xz" => Err(anyhow!("This binary was not compiled with the xz feature enabled! Cannot read {path:?}.")),

        // If the extension is not known 
        unknown => {
            event!(
                    Level::WARN,
                    "Unknown GeoNames file extension '{}', falling back to plain text! Supported extensions are: {}",
                    unknown,
                    [
                        "txt",
                        #[cfg(feature = "bzip2")]
                        "bz2",
                        #[cfg(feature = "gzip")]
                        "gz",
                        #[cfg(feature = "xz")]
                        "xz",
                    ].join(", ")
                );
            Ok(Box::new(buf_reader))
        }
    }
}

pub(crate) fn parse_geonames_file(
    path: &str,
    query_pairs: &mut Vec<(String, MatchType)>,
    geonames: &mut HashMap<u64, GeoNamesEntry>,
) -> Result<(), anyhow::Error> {
    let reader: Box<dyn Read> = get_reader(Path::new(path))?;

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(reader);

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
        let adm1 = record.get(10).unwrap_or("").to_string();
        let adm2 = record.get(11).unwrap_or("").to_string();
        let adm3 = record.get(12).unwrap_or("").to_string();
        let adm4 = record.get(13).unwrap_or("").to_string();
        let elevation: Option<i16> = record.get(15).and_then(|i| i.parse().ok());

        if name_ascii != name {
            query_pairs.push((name_ascii, MatchType::AsciiName { id }));
        }
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
                adm1,
                adm2,
                adm3,
                adm4,
                elevation,
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
    let reader: Box<dyn Read> = get_reader(Path::new(path))?;

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(reader);

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
