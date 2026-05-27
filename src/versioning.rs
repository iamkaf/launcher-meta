use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::HashMap;

pub fn latest_forge_version(metadata: &str, minecraft: &str) -> Option<String> {
    let manifest: HashMap<String, Vec<String>> = serde_json::from_str(metadata).ok()?;
    manifest
        .get(minecraft)?
        .iter()
        .max_by(|left, right| compare_loader_versions(left, right))
        .cloned()
}

#[derive(Clone, Copy, Debug)]
pub struct NeoForgeSource {
    pub legacy_1201: bool,
    pub artifact_kind: &'static str,
}

pub fn neoforge_source(minecraft: &str) -> NeoForgeSource {
    if minecraft == "1.20.1" {
        NeoForgeSource {
            legacy_1201: true,
            artifact_kind: "neoforge-legacy",
        }
    } else {
        NeoForgeSource {
            legacy_1201: false,
            artifact_kind: "neoforge",
        }
    }
}

pub fn latest_neoforge_version(
    metadata: &str,
    source: NeoForgeSource,
    minecraft: &str,
) -> Option<String> {
    maven_metadata_versions(metadata)
        .into_iter()
        .filter(|version| {
            source.legacy_1201 || neoforge_minecraft_version(version).as_deref() == Some(minecraft)
        })
        .max_by(|left, right| compare_loader_versions(left, right))
}

pub fn maven_metadata_versions(metadata: &str) -> Vec<String> {
    metadata
        .split("<version>")
        .skip(1)
        .filter_map(|part| {
            part.split_once("</version>")
                .map(|(version, _)| version.trim())
        })
        .filter(|version| !version.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn latest_maven_version(metadata: &str) -> Option<String> {
    find_xml_tag(metadata, "latest").or_else(|| {
        maven_metadata_versions(metadata)
            .into_iter()
            .max_by(|left, right| compare_loader_versions(left, right))
    })
}

pub fn find_xml_tag(metadata: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    metadata
        .split(&open)
        .nth(1)?
        .split_once(&close)
        .map(|(value, _)| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn neoforge_minecraft_version(version: &str) -> Option<String> {
    if version.contains("25w") {
        let snapshot = version
            .trim_start_matches("0.")
            .split('.')
            .next()
            .filter(|part| !part.is_empty())?;
        return Some(format!("1.0.{snapshot}"));
    }

    let core = version.split('-').next().unwrap_or(version);
    let mut parts = core.split('.');
    let major_or_year = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next()?;

    if major_or_year >= 26 {
        let hotfix = parts.next()?;
        if hotfix == "0" {
            Some(format!("{major_or_year}.{minor}"))
        } else {
            Some(format!("{major_or_year}.{minor}.{hotfix}"))
        }
    } else if minor == "0" {
        Some(format!("1.{major_or_year}"))
    } else {
        Some(format!("1.{major_or_year}.{minor}"))
    }
}

pub fn compare_loader_versions(left: &str, right: &str) -> Ordering {
    loader_version_key(left).cmp(&loader_version_key(right))
}

fn loader_version_key(version: &str) -> Vec<u32> {
    loader_version_for_ordering(version)
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<u32>().unwrap_or(0))
        .collect()
}

pub fn loader_version_for_ordering(version: &str) -> &str {
    if let Some((minecraft, loader)) = version.split_once('-')
        && minecraft
            .chars()
            .all(|char| char.is_ascii_digit() || char == '.')
        && loader
            .chars()
            .next()
            .is_some_and(|char| char.is_ascii_digit())
    {
        loader
    } else {
        version
    }
}

pub fn installer_artifact_version(loader: &str, minecraft: &str, version: &str) -> String {
    if loader == "forge" && !version.contains('-') {
        format!("{minecraft}-{version}")
    } else {
        version.to_string()
    }
}

pub fn sort_semverish(mut versions: Vec<String>) -> Vec<String> {
    versions.sort_by(|left, right| compare_loader_versions(left, right));
    versions
}

#[derive(Debug, Deserialize)]
pub struct FabricMetadata {
    pub loader: FabricComponent,
}

#[derive(Debug, Deserialize)]
pub struct FabricComponent {
    pub maven: String,
    pub version: Option<String>,
}

pub fn parse_fabric_metadata(metadata: &str) -> Result<FabricMetadata, String> {
    match serde_json::from_str::<FabricMetadata>(metadata) {
        Ok(metadata) => Ok(metadata),
        Err(object_error) => {
            let mut versions: Vec<FabricMetadata> = serde_json::from_str(metadata)
                .map_err(|_| format!("failed to parse Fabric metadata: {object_error}"))?;
            versions
                .drain(..)
                .next()
                .ok_or_else(|| "Fabric metadata did not include any versions".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_latest_forge_version_for_minecraft() {
        let metadata = r#"{
            "1.21.1": ["52.0.1", "52.1.0"],
            "1.20.1": ["47.1.82", "1.20.1-47.1.106"]
        }"#;

        assert_eq!(
            latest_forge_version(metadata, "1.21.1").as_deref(),
            Some("52.1.0")
        );
        assert_eq!(
            latest_forge_version(metadata, "1.20.1").as_deref(),
            Some("1.20.1-47.1.106")
        );
    }

    #[test]
    fn selects_latest_neoforge_modern_and_legacy_versions() {
        let modern = r#"<metadata><versioning><versions>
<version>21.1.231</version>
<version>26.1.2.21-beta</version>
<version>26.1.2.66-beta</version>
</versions></versioning></metadata>"#;
        let legacy = r#"<metadata><versioning><versions>
<version>47.1.82</version>
<version>1.20.1-47.1.106</version>
</versions></versioning></metadata>"#;

        assert_eq!(
            latest_neoforge_version(modern, neoforge_source("26.1.2"), "26.1.2").as_deref(),
            Some("26.1.2.66-beta")
        );
        assert_eq!(
            latest_neoforge_version(legacy, neoforge_source("1.20.1"), "1.20.1").as_deref(),
            Some("1.20.1-47.1.106")
        );
    }

    #[test]
    fn parses_fabric_object_and_array_metadata() {
        let object =
            r#"{"loader":{"maven":"net.fabricmc:fabric-loader:0.16.14","version":"0.16.14"}}"#;
        let array = format!("[{object}]");

        assert_eq!(
            parse_fabric_metadata(object)
                .unwrap()
                .loader
                .version
                .as_deref(),
            Some("0.16.14")
        );
        assert_eq!(
            parse_fabric_metadata(&array).unwrap().loader.maven,
            "net.fabricmc:fabric-loader:0.16.14"
        );
    }

    #[test]
    fn maps_neoforge_versions_to_minecraft_versions() {
        assert_eq!(
            neoforge_minecraft_version("21.1.231").as_deref(),
            Some("1.21.1")
        );
        assert_eq!(
            neoforge_minecraft_version("26.1.2.66-beta").as_deref(),
            Some("26.1.2")
        );
        assert_eq!(
            neoforge_minecraft_version("0.25w14craftmine.5-beta").as_deref(),
            Some("1.0.25w14craftmine")
        );
    }
}
