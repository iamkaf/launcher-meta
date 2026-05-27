use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ItemStatus {
    Ok,
    Error,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MinecraftVersion {
    pub id: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoaderItem {
    pub loader: String,
    pub status: ItemStatus,
    pub version: Option<String>,
    pub maven: Option<String>,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyItem {
    pub id: String,
    pub kind: String,
    pub status: ItemStatus,
    pub version: Option<String>,
    pub loader_versions: LoaderVersions,
    pub coordinates: Option<String>,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoaderVersions {
    pub forge: Option<String>,
    pub neoforge: Option<String>,
    pub fabric: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionsData {
    pub versions: Vec<MinecraftVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoadersData {
    pub minecraft: String,
    pub loaders: Vec<LoaderItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependenciesData {
    pub minecraft: String,
    pub dependencies: Vec<DependencyItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompatibilityData {
    pub projects: BTreeMap<String, BTreeMap<String, LoaderVersions>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthData {
    pub status: String,
    pub upstream: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_at: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T, timestamp: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: timestamp.clone(),
            cached_at: Some(timestamp),
        }
    }

    pub fn error(message: impl Into<String>, timestamp: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
            timestamp,
            cached_at: None,
        }
    }
}
