use std::collections::BTreeSet;

use crate::config::{MINECRAFT_THURSDAY_TTL, MINECRAFT_TUESDAY_TTL, MINECRAFT_WEDNESDAY_TTL};

pub fn normalize_list(input: Option<&str>, defaults: &[&str]) -> Vec<String> {
    let values: Vec<String> = match input {
        Some(raw) => raw
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect(),
        None => defaults.iter().map(|value| value.to_string()).collect(),
    };

    unique_preserving_order(values)
}

pub fn unique_preserving_order(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            out.push(value);
        }
    }
    out
}

pub fn sorted_cache_list(values: &[String]) -> String {
    if values.is_empty() {
        return "_".to_string();
    }
    let mut sorted = values.to_vec();
    sorted.sort();
    sorted.join(",")
}

pub fn dependency_cache_key(minecraft: &str, mods: &[String]) -> String {
    format!("dependencies/{minecraft}?mods={}", sorted_cache_list(mods))
}

pub fn compatibility_cache_key(mods: &[String], minecraft_versions: &[String]) -> String {
    format!(
        "compatibility?mods={}&minecraft={}",
        sorted_cache_list(mods),
        sorted_cache_list(minecraft_versions)
    )
}

pub fn rate_limit_key(client: &str, bucket: &str) -> String {
    format!("{bucket}:{}", client.trim().to_ascii_lowercase())
}

pub fn validate_minecraft(version: &str) -> Result<(), String> {
    if version.is_empty() || version.len() > 64 {
        return Err("Minecraft version must be non-empty and at most 64 characters".to_string());
    }

    if !version
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    {
        return Err("Minecraft version contains unsupported characters".to_string());
    }

    Ok(())
}

pub fn minecraft_manifest_ttl_for_utc_day(utc_day: u32, normal_ttl: u64) -> u64 {
    match utc_day {
        2 => MINECRAFT_TUESDAY_TTL,
        3 => MINECRAFT_WEDNESDAY_TTL,
        4 => MINECRAFT_THURSDAY_TTL,
        _ => normal_ttl,
    }
}

#[cfg(target_arch = "wasm32")]
pub fn current_utc_day() -> u32 {
    js_sys::Date::new_0().get_utc_day()
}

#[cfg(target_arch = "wasm32")]
pub fn now_iso() -> String {
    js_sys::Date::new_0().to_iso_string().into()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn now_iso() -> String {
    "1970-01-01T00:00:00.000Z".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_sorts_cache_lists() {
        let values = normalize_list(Some("modmenu, amber,modmenu"), &[]);
        assert_eq!(values, vec!["modmenu", "amber"]);
        assert_eq!(sorted_cache_list(&values), "amber,modmenu");
    }

    #[test]
    fn builds_order_independent_cache_keys() {
        let left = vec!["modmenu".to_string(), "amber".to_string()];
        let right = vec!["amber".to_string(), "modmenu".to_string()];
        assert_eq!(
            dependency_cache_key("1.21.4", &left),
            dependency_cache_key("1.21.4", &right)
        );
        assert!(dependency_cache_key("1.21.4", &left).contains("mods="));

        let versions_left = vec!["1.21.4".to_string(), "1.20.1".to_string()];
        let versions_right = vec!["1.20.1".to_string(), "1.21.4".to_string()];
        assert_eq!(
            compatibility_cache_key(&left, &versions_left),
            compatibility_cache_key(&right, &versions_right)
        );
        assert!(compatibility_cache_key(&left, &versions_left).contains("mods="));
    }

    #[test]
    fn validates_minecraft_versions() {
        assert!(validate_minecraft("1.21.4").is_ok());
        assert!(validate_minecraft("25w14craftmine").is_ok());
        assert!(validate_minecraft("../bad").is_err());
    }

    #[test]
    fn minecraft_manifest_ttl_tracks_likely_release_days() {
        assert_eq!(minecraft_manifest_ttl_for_utc_day(2, 21_600), 300);
        assert_eq!(minecraft_manifest_ttl_for_utc_day(3, 21_600), 600);
        assert_eq!(minecraft_manifest_ttl_for_utc_day(4, 21_600), 1_800);
        assert_eq!(minecraft_manifest_ttl_for_utc_day(5, 21_600), 21_600);
    }

    #[test]
    fn builds_stable_rate_limit_keys() {
        assert_eq!(rate_limit_key(" 203.0.113.10 ", "api"), "api:203.0.113.10");
    }
}
