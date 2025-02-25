use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct GeoNamesEntry {
    pub id: u64,
    pub name: String,
    pub latitude: f32,
    pub longitude: f32,
    pub feature_class: String,
    pub feature_code: String,
    pub country_code: String,
}

#[derive(Debug, Serialize, PartialEq, JsonSchema)]
pub struct GeoNamesSearchResult {
    key: MatchKey,
    entry: GeoNamesEntry,
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
struct MatchKey {
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
