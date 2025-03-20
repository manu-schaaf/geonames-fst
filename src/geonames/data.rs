use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct GeoNamesEntry {
    /// Unique identifier of the record
    pub id: u64,
    /// Canonical name of the entry, usually English.
    pub name: String,
    /// Latitude of the GeoNames record
    pub latitude: f32,
    /// Longitude of the GeoNames record
    pub longitude: f32,
    /// Feature class of the GeoNames record
    pub feature_class: String,
    /// Feature code of the GeoNames record
    pub feature_code: String,
    /// Country code of the GeoNames record
    pub country_code: String,
    /// Administrative divisions of the GeoNames record, some of which may be empty.
    pub administrative_divisions: (String, String, String, String),
    /// Elevation of the GeoNames record, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elevation: Option<i16>,
}

pub trait Entry {
    fn entry(&self) -> &GeoNamesEntry;
}

#[derive(Debug, Serialize, PartialEq, JsonSchema)]
pub struct GeoNamesSearchResult {
    pub key: MatchKey,
    pub entry: GeoNamesEntry,
}

impl GeoNamesSearchResult {
    pub fn new(key: &str, typ: &MatchType, gn: &GeoNamesEntry) -> Self {
        GeoNamesSearchResult {
            key: MatchKey {
                name: key.to_string(),
                typ: typ.clone(),
            },
            entry: gn.clone(),
        }
    }
}

impl Entry for GeoNamesSearchResult {
    fn entry(&self) -> &GeoNamesEntry {
        &self.entry
    }
}

impl Eq for GeoNamesSearchResult {}

impl Ord for GeoNamesSearchResult {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

impl PartialOrd for GeoNamesSearchResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl From<GeoNamesSearchResult> for GeoNamesSearchResultWithDist {
    fn from(val: GeoNamesSearchResult) -> Self {
        GeoNamesSearchResultWithDist {
            key: val.key,
            entry: val.entry,
            distance: 0,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct GeoNamesSearchResultWithDist {
    key: MatchKey,
    entry: GeoNamesEntry,
    distance: usize,
}

impl GeoNamesSearchResultWithDist {
    pub fn new(key: &str, typ: &MatchType, gn: &GeoNamesEntry, dist: usize) -> Self {
        GeoNamesSearchResultWithDist {
            key: MatchKey {
                name: key.to_string(),
                typ: typ.clone(),
            },
            entry: gn.clone(),
            distance: dist,
        }
    }
}

impl Entry for GeoNamesSearchResultWithDist {
    fn entry(&self) -> &GeoNamesEntry {
        &self.entry
    }
}

impl Eq for GeoNamesSearchResultWithDist {}

impl Ord for GeoNamesSearchResultWithDist {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let cmp = self.distance.cmp(&other.distance);
        if cmp.is_eq() {
            self.key.cmp(&other.key)
        } else {
            cmp
        }
    }
}

impl PartialOrd for GeoNamesSearchResultWithDist {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, JsonSchema)]
#[serde(tag = "type")]
pub enum MatchType {
    /// GeoNames main name (usually English)
    Name { id: u64 },
    /// ASCII version of the main name
    AsciiName { id: u64 },
    /// Alternate: preferred name in a specific language
    PreferredName { id: u64, lang: String },
    /// Alternate: short name in a specific language
    ShortName { id: u64, lang: String },
    /// Alternate: colloquial name or slang in a specific language
    Colloquial { id: u64, lang: String },
    /// Alternate: historic name in a specific language
    Historic {
        id: u64,
        lang: String,
        from: String,
        to: String,
    },
    /// Alternate: other name in a specific language
    Alternate { id: u64, lang: String },
}

impl MatchType {
    pub(crate) fn id(&self) -> u64 {
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

    pub(crate) fn ord(&self) -> u8 {
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
        let cmp = self.ord().cmp(&other.ord());
        if cmp.is_eq() {
            self.id().cmp(&other.id())
        } else {
            cmp
        }
    }
}

impl PartialOrd for MatchType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Serialize, PartialEq, Eq, JsonSchema)]
pub struct MatchKey {
    name: String,
    #[serde(flatten)]
    typ: MatchType,
}

impl PartialOrd for MatchKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MatchKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.typ.cmp(&other.typ)
    }
}
