use crate::config::*;
use crate::types::*;
use crate::util::unique_preserving_order;
use crate::versioning::*;
use futures_util::future::join_all;
use serde::Deserialize;
use std::collections::BTreeMap;

#[cfg(target_arch = "wasm32")]
use worker::{AbortSignal, Fetch, Headers, Method, Request, RequestInit};

#[derive(Clone, Debug, Default)]
pub struct UpstreamConfig {
    pub modrinth_token: Option<String>,
}

#[cfg(target_arch = "wasm32")]
async fn fetch_text(url: &str, config: &UpstreamConfig) -> Result<String, String> {
    let headers = Headers::new();
    headers
        .set("User-Agent", USER_AGENT)
        .map_err(|error| error.to_string())?;
    headers
        .set(
            "Accept",
            "application/json, text/xml, application/xml, text/plain;q=0.8",
        )
        .map_err(|error| error.to_string())?;
    if url.starts_with("https://api.modrinth.com/") {
        if let Some(token) = config.modrinth_token.as_deref() {
            headers
                .set("Authorization", &format!("Bearer {token}"))
                .map_err(|error| error.to_string())?;
        }
    }

    let mut init = RequestInit::new();
    init.with_method(Method::Get);
    init.with_headers(headers);

    let request = Request::new_with_init(url, &init).map_err(|error| error.to_string())?;
    let signal = AbortSignal::from(web_sys::AbortSignal::timeout_with_u32(UPSTREAM_TIMEOUT_MS));
    let mut response = Fetch::Request(request)
        .send_with_signal(&signal)
        .await
        .map_err(|error| error.to_string())?;

    if !(200..=299).contains(&response.status_code()) {
        return Err(format!("upstream returned HTTP {}", response.status_code()));
    }

    response.text().await.map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_health(url: &str, config: &UpstreamConfig) -> String {
    match fetch_text(url, config).await {
        Ok(_) => "ok".to_string(),
        Err(_) => "error".to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
pub async fn minecraft_versions(config: &UpstreamConfig) -> Result<VersionsData, String> {
    #[derive(Deserialize)]
    struct Manifest {
        versions: Vec<ManifestVersion>,
    }

    #[derive(Deserialize)]
    struct ManifestVersion {
        id: String,
        #[serde(rename = "type")]
        kind: String,
    }

    let body = fetch_text(MOJANG_MANIFEST_URL, config).await?;
    let manifest: Manifest = serde_json::from_str(&body).map_err(|error| error.to_string())?;
    Ok(VersionsData {
        versions: manifest
            .versions
            .into_iter()
            .map(|version| MinecraftVersion {
                id: version.id,
                kind: version.kind,
            })
            .collect(),
    })
}

#[cfg(target_arch = "wasm32")]
pub async fn loaders_for_minecraft(minecraft: &str, config: &UpstreamConfig) -> LoadersData {
    let (fabric, forge, neoforge) = futures_util::join!(
        resolve_fabric_loader(minecraft, config),
        resolve_forge_loader(minecraft, config),
        resolve_neoforge_loader(minecraft, config)
    );

    LoadersData {
        minecraft: minecraft.to_string(),
        loaders: vec![fabric, forge, neoforge],
    }
}

#[cfg(target_arch = "wasm32")]
async fn resolve_fabric_loader(minecraft: &str, config: &UpstreamConfig) -> LoaderItem {
    let url = format!("{FABRIC_LOADER_BASE_URL}/{minecraft}");
    match fetch_text(&url, config)
        .await
        .and_then(|body| parse_fabric_metadata(&body))
    {
        Ok(metadata) => LoaderItem {
            loader: "fabric".to_string(),
            status: ItemStatus::Ok,
            version: metadata.loader.version,
            maven: Some(metadata.loader.maven),
            source: url,
            error: None,
        },
        Err(error) => loader_error("fabric", &url, error),
    }
}

#[cfg(target_arch = "wasm32")]
async fn resolve_forge_loader(minecraft: &str, config: &UpstreamConfig) -> LoaderItem {
    match fetch_text(FORGE_METADATA_URL, config)
        .await
        .and_then(|body| {
            latest_forge_version(&body, minecraft)
                .ok_or_else(|| format!("no Forge loader found for Minecraft {minecraft}"))
        }) {
        Ok(version) => {
            let artifact = installer_artifact_version("forge", minecraft, &version);
            LoaderItem {
                loader: "forge".to_string(),
                status: ItemStatus::Ok,
                version: Some(artifact.clone()),
                maven: Some(format!("net.minecraftforge:forge:{artifact}:installer")),
                source: FORGE_METADATA_URL.to_string(),
                error: None,
            }
        }
        Err(error) => loader_error("forge", FORGE_METADATA_URL, error),
    }
}

#[cfg(target_arch = "wasm32")]
async fn resolve_neoforge_loader(minecraft: &str, config: &UpstreamConfig) -> LoaderItem {
    let source = neoforge_source(minecraft);
    let url = if source.legacy_1201 {
        NEOFORGE_LEGACY_METADATA_URL
    } else {
        NEOFORGE_METADATA_URL
    };

    match fetch_text(url, config).await.and_then(|body| {
        latest_neoforge_version(&body, source, minecraft)
            .ok_or_else(|| format!("no NeoForge loader found for Minecraft {minecraft}"))
    }) {
        Ok(version) => {
            let group_artifact = if source.artifact_kind == "neoforge-legacy" {
                "net.neoforged:forge"
            } else {
                "net.neoforged:neoforge"
            };
            LoaderItem {
                loader: "neoforge".to_string(),
                status: ItemStatus::Ok,
                version: Some(version.clone()),
                maven: Some(format!("{group_artifact}:{version}:installer")),
                source: url.to_string(),
                error: None,
            }
        }
        Err(error) => loader_error("neoforge", url, error),
    }
}

fn loader_error(loader: &str, source: &str, error: String) -> LoaderItem {
    LoaderItem {
        loader: loader.to_string(),
        status: ItemStatus::Error,
        version: None,
        maven: None,
        source: source.to_string(),
        error: Some(error),
    }
}

#[cfg(target_arch = "wasm32")]
pub async fn dependencies_for_minecraft(
    minecraft: &str,
    mods: &[String],
    config: &UpstreamConfig,
) -> DependenciesData {
    let mut ids: Vec<String> = BUILT_INS.iter().map(|id| id.to_string()).collect();
    ids.extend(mods.iter().cloned());
    ids = unique_preserving_order(ids);

    let futures = ids
        .iter()
        .map(|id| dependency_for_minecraft(id, minecraft, config));
    let dependencies = join_all(futures).await;
    DependenciesData {
        minecraft: minecraft.to_string(),
        dependencies,
    }
}

#[cfg(target_arch = "wasm32")]
async fn dependency_for_minecraft(
    id: &str,
    minecraft: &str,
    config: &UpstreamConfig,
) -> DependencyItem {
    match id {
        "forge" => loader_dependency(
            "forge",
            "loader",
            resolve_forge_loader(minecraft, config).await,
        ),
        "neoforge" => loader_dependency(
            "neoforge",
            "loader",
            resolve_neoforge_loader(minecraft, config).await,
        ),
        "fabric-loader" => loader_dependency(
            "fabric-loader",
            "loader",
            resolve_fabric_loader(minecraft, config).await,
        ),
        "parchment" => resolve_parchment(minecraft, config).await,
        "neoform" => {
            resolve_prefixed_maven(
                "neoform",
                "mapping",
                NEOFORM_METADATA_URL,
                &format!("{minecraft}-"),
                "https://maven.neoforged.net/releases/net/neoforged/neoform/",
                None,
                config,
            )
            .await
        }
        "forgegradle" => {
            resolve_latest_maven(
                "forgegradle",
                "tool",
                FORGEGRADLE_METADATA_URL,
                "https://plugins.gradle.org/plugin/net.minecraftforge.gradle",
                config,
            )
            .await
        }
        "moddev-gradle" => {
            resolve_latest_maven(
                "moddev-gradle",
                "tool",
                MODDEV_GRADLE_METADATA_URL,
                "https://maven.neoforged.net/releases/net/neoforged/moddev-gradle/",
                config,
            )
            .await
        }
        "loom" => resolve_loom(minecraft, config).await,
        project => resolve_modrinth_project(project, minecraft, config).await,
    }
}

fn loader_dependency(id: &str, kind: &str, loader: LoaderItem) -> DependencyItem {
    let mut loader_versions = LoaderVersions::default();
    match loader.loader.as_str() {
        "forge" => loader_versions.forge = loader.version.clone(),
        "neoforge" => loader_versions.neoforge = loader.version.clone(),
        "fabric" => loader_versions.fabric = loader.version.clone(),
        _ => {}
    }

    DependencyItem {
        id: id.to_string(),
        kind: kind.to_string(),
        status: loader.status,
        version: loader.version,
        loader_versions,
        coordinates: loader.maven,
        source: loader.source,
        error: loader.error,
    }
}

#[cfg(target_arch = "wasm32")]
async fn resolve_parchment(minecraft: &str, config: &UpstreamConfig) -> DependencyItem {
    if is_unobfuscated_minecraft(minecraft) {
        return DependencyItem {
            id: "parchment".to_string(),
            kind: "mapping".to_string(),
            status: ItemStatus::Unavailable,
            version: None,
            loader_versions: LoaderVersions::default(),
            coordinates: None,
            source: "https://maven.parchmentmc.org/".to_string(),
            error: None,
        };
    }

    let candidates = parchment_candidates(minecraft);
    for candidate in candidates {
        let url = PARCHMENT_BASE_URL.replace("{version}", &candidate);
        let Ok(body) = fetch_text(&url, config).await else {
            continue;
        };
        let versions: Vec<String> = maven_metadata_versions(&body)
            .into_iter()
            .filter(|version| !version.contains("nightly") && !version.contains("SNAPSHOT"))
            .collect();
        let Some(version) = sort_semverish(versions).pop() else {
            continue;
        };
        return DependencyItem {
            id: "parchment".to_string(),
            kind: "mapping".to_string(),
            status: ItemStatus::Ok,
            version: Some(version),
            loader_versions: LoaderVersions::default(),
            coordinates: None,
            source: url,
            error: None,
        };
    }
    dependency_error(
        "parchment",
        "mapping",
        "https://maven.parchmentmc.org/",
        "no Parchment version found",
    )
}

fn parchment_candidates(minecraft: &str) -> Vec<String> {
    let mut candidates = vec![minecraft.to_string()];
    let parts: Vec<&str> = minecraft.split('.').collect();
    if parts.len() >= 3 {
        let major = parts[0];
        let minor = parts[1];
        let patch = parts[2]
            .split('-')
            .next()
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(0);
        for patch in (0..patch).rev() {
            candidates.push(format!("{major}.{minor}.{patch}"));
        }
    }
    candidates
}

fn is_unobfuscated_minecraft(minecraft: &str) -> bool {
    let core = minecraft.split('-').next().unwrap_or(minecraft);
    let mut parts = core.split('.');
    let Some(major) = parts.next().and_then(|value| value.parse::<u32>().ok()) else {
        return false;
    };
    let Some(minor) = parts.next().and_then(|value| value.parse::<u32>().ok()) else {
        return false;
    };

    major > 26 || (major == 26 && minor >= 1)
}

#[cfg(target_arch = "wasm32")]
async fn resolve_prefixed_maven(
    id: &str,
    kind: &str,
    metadata_url: &str,
    prefix: &str,
    source: &str,
    coordinates_group: Option<&str>,
    config: &UpstreamConfig,
) -> DependencyItem {
    match fetch_text(metadata_url, config).await {
        Ok(body) => {
            let versions: Vec<String> = maven_metadata_versions(&body)
                .into_iter()
                .filter(|version| version.starts_with(prefix))
                .collect();
            match sort_semverish(versions).pop() {
                Some(version) => DependencyItem {
                    id: id.to_string(),
                    kind: kind.to_string(),
                    status: ItemStatus::Ok,
                    coordinates: coordinates_group.map(|group| format!("{group}:{version}")),
                    version: Some(version),
                    loader_versions: LoaderVersions::default(),
                    source: source.to_string(),
                    error: None,
                },
                None => dependency_error(id, kind, source, "no matching Maven version found"),
            }
        }
        Err(error) => dependency_error(id, kind, source, error),
    }
}

#[cfg(target_arch = "wasm32")]
async fn resolve_latest_maven(
    id: &str,
    kind: &str,
    metadata_url: &str,
    source: &str,
    config: &UpstreamConfig,
) -> DependencyItem {
    match fetch_text(metadata_url, config).await {
        Ok(body) => match latest_maven_version(&body) {
            Some(version) => DependencyItem {
                id: id.to_string(),
                kind: kind.to_string(),
                status: ItemStatus::Ok,
                version: Some(version),
                loader_versions: LoaderVersions::default(),
                coordinates: None,
                source: source.to_string(),
                error: None,
            },
            None => dependency_error(id, kind, source, "no Maven version found"),
        },
        Err(error) => dependency_error(id, kind, source, error),
    }
}

#[cfg(target_arch = "wasm32")]
async fn resolve_loom(_minecraft: &str, config: &UpstreamConfig) -> DependencyItem {
    match fetch_text(LOOM_METADATA_URL, config).await {
        Ok(body) => {
            let versions: Vec<String> = maven_metadata_versions(&body)
                .into_iter()
                .filter(|version| version.contains("SNAPSHOT"))
                .collect();
            match sort_semverish(versions).pop() {
                Some(version) => DependencyItem {
                    id: "loom".to_string(),
                    kind: "tool".to_string(),
                    status: ItemStatus::Ok,
                    version: Some(version.clone()),
                    loader_versions: LoaderVersions {
                        fabric: Some(version),
                        ..LoaderVersions::default()
                    },
                    coordinates: None,
                    source: "https://maven.fabricmc.net/net/fabricmc/fabric-loom/".to_string(),
                    error: None,
                },
                None => dependency_error(
                    "loom",
                    "tool",
                    "https://maven.fabricmc.net/net/fabricmc/fabric-loom/",
                    "no Loom SNAPSHOT version found",
                ),
            }
        }
        Err(error) => dependency_error(
            "loom",
            "tool",
            "https://maven.fabricmc.net/net/fabricmc/fabric-loom/",
            error,
        ),
    }
}

#[derive(Debug, Deserialize)]
struct ModrinthProject {
    slug: String,
}

#[derive(Debug, Deserialize)]
struct ModrinthVersion {
    version_number: String,
    date_published: String,
    loaders: Vec<String>,
}

#[cfg(target_arch = "wasm32")]
async fn resolve_modrinth_project(
    project: &str,
    minecraft: &str,
    config: &UpstreamConfig,
) -> DependencyItem {
    let encoded_versions = format!("%5B%22{minecraft}%22%5D");
    let versions_url =
        format!("{MODRINTH_PROJECT_BASE_URL}/{project}/version?game_versions={encoded_versions}");
    let project_url = format!("{MODRINTH_PROJECT_BASE_URL}/{project}");
    let source = format!("https://modrinth.com/mod/{project}");

    let (versions_body, project_body) = futures_util::join!(
        fetch_text(&versions_url, config),
        fetch_text(&project_url, config)
    );

    let versions_body = match versions_body {
        Ok(body) => body,
        Err(error) => return dependency_error(project, "mod", &source, error),
    };
    let versions: Vec<ModrinthVersion> = match serde_json::from_str(&versions_body) {
        Ok(versions) => versions,
        Err(error) => return dependency_error(project, "mod", &source, error.to_string()),
    };
    let Some(latest) = versions
        .iter()
        .max_by_key(|version| &version.date_published)
    else {
        return dependency_error(project, "mod", &source, "no Modrinth versions found");
    };

    let mut loader_versions = LoaderVersions::default();
    for loader in ["forge", "neoforge", "fabric"] {
        if let Some(version) = versions
            .iter()
            .filter(|version| version.loaders.iter().any(|candidate| candidate == loader))
            .max_by_key(|version| &version.date_published)
        {
            match loader {
                "forge" => loader_versions.forge = Some(version.version_number.clone()),
                "neoforge" => loader_versions.neoforge = Some(version.version_number.clone()),
                "fabric" => loader_versions.fabric = Some(version.version_number.clone()),
                _ => {}
            }
        }
    }

    let id = project_body
        .ok()
        .and_then(|body| serde_json::from_str::<ModrinthProject>(&body).ok())
        .map(|project| project.slug)
        .unwrap_or_else(|| project.to_string());

    DependencyItem {
        id,
        kind: "mod".to_string(),
        status: ItemStatus::Ok,
        version: Some(latest.version_number.clone()),
        loader_versions,
        coordinates: Some(format!(
            "maven.modrinth:{project}:{}",
            latest.version_number
        )),
        source,
        error: None,
    }
}

fn dependency_error(
    id: &str,
    kind: &str,
    source: &str,
    error: impl Into<String>,
) -> DependencyItem {
    DependencyItem {
        id: id.to_string(),
        kind: kind.to_string(),
        status: ItemStatus::Error,
        version: None,
        loader_versions: LoaderVersions::default(),
        coordinates: None,
        source: source.to_string(),
        error: Some(error.into()),
    }
}

#[cfg(target_arch = "wasm32")]
pub async fn compatibility(
    mods: &[String],
    minecraft_versions: &[String],
    config: &UpstreamConfig,
) -> CompatibilityData {
    let mut out: BTreeMap<String, BTreeMap<String, LoaderVersions>> = BTreeMap::new();
    let futures = mods.iter().flat_map(|modrinth_mod| {
        minecraft_versions.iter().map(move |minecraft| async move {
            let item = dependency_for_minecraft(modrinth_mod, minecraft, config).await;
            (
                modrinth_mod.clone(),
                minecraft.clone(),
                item.loader_versions,
            )
        })
    });

    for (modrinth_mod, minecraft, loaders) in join_all(futures).await {
        out.entry(modrinth_mod)
            .or_default()
            .insert(minecraft, loaders);
    }

    CompatibilityData { mods: out }
}

#[cfg(test)]
pub fn compatibility_nulls(mods: &[String], minecraft_versions: &[String]) -> CompatibilityData {
    let mut out = BTreeMap::new();
    for modrinth_mod in mods {
        let mut versions = BTreeMap::new();
        for minecraft in minecraft_versions {
            versions.insert(minecraft.clone(), LoaderVersions::default());
        }
        out.insert(modrinth_mod.clone(), versions);
    }
    CompatibilityData { mods: out }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_failure_serializes_as_error_item() {
        let item = dependency_error(
            "bad-project",
            "mod",
            "https://modrinth.com/mod/bad-project",
            "not found",
        );
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains(r#""status":"error""#));
        assert!(json.contains(r#""error":"not found""#));
    }

    #[test]
    fn compatibility_null_shape_contains_requested_entries() {
        let mods = vec!["amber".to_string()];
        let versions = vec!["1.21.1".to_string(), "1.21.4".to_string()];
        let data = compatibility_nulls(&mods, &versions);
        assert!(data.mods["amber"]["1.21.1"].fabric.is_none());
        assert!(data.mods["amber"]["1.21.4"].forge.is_none());
    }

    #[test]
    fn detects_unobfuscated_minecraft_versions() {
        assert!(!is_unobfuscated_minecraft("1.21.4"));
        assert!(!is_unobfuscated_minecraft("26.0"));
        assert!(is_unobfuscated_minecraft("26.1"));
        assert!(is_unobfuscated_minecraft("26.1.2"));
        assert!(is_unobfuscated_minecraft("26.2-pre-1"));
        assert!(is_unobfuscated_minecraft("27.0"));
    }
}
