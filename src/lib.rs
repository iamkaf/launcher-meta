#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]
#![recursion_limit = "256"]

mod config;
mod docs;
mod service;
mod types;
mod util;
mod versioning;

#[cfg(target_arch = "wasm32")]
use crate::config::*;
#[cfg(target_arch = "wasm32")]
use crate::types::*;
#[cfg(target_arch = "wasm32")]
use crate::util::{
    compatibility_cache_key, current_utc_day, dependency_cache_key,
    minecraft_manifest_ttl_for_utc_day, normalize_list, now_iso, rate_limit_key,
    validate_compatibility_minecraft_versions, validate_minecraft, validate_mods,
    validate_query_keys,
};
#[cfg(target_arch = "wasm32")]
use std::collections::BTreeMap;
#[cfg(target_arch = "wasm32")]
use worker::*;

#[cfg(target_arch = "wasm32")]
#[event(start)]
fn start() {
    console_error_panic_hook::set_once();
}

#[cfg(target_arch = "wasm32")]
#[event(fetch)]
async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    if req.method() == Method::Options {
        return options_response();
    }
    if req.method() != Method::Get {
        return method_not_allowed_response();
    }

    let path = req.path();
    let route = path.trim_matches('/').split('/').collect::<Vec<_>>();
    if route.first() != Some(&"v1") {
        return json_error("not found", 404);
    }

    if let Some(response) = enforce_rate_limits(&req, &env).await? {
        return Ok(response);
    }

    let upstream = upstream_config(&env);
    let response = match route.as_slice() {
        ["v1"] => {
            if let Err(error) = validate_request_query(&req, &[]) {
                return json_error(&error, 400);
            }
            json_ok(ApiResponse::success(docs::api_docs(), now_iso()))
        }
        ["v1", "health"] => {
            if let Err(error) = validate_request_query(&req, &[]) {
                return json_error(&error, 400);
            }
            health(&upstream).await
        }
        ["v1", "minecraft", "versions"] => {
            if let Err(error) = validate_request_query(&req, &[]) {
                return json_error(&error, 400);
            }
            let upstream = upstream.clone();
            cached(
                req,
                &env,
                &ctx,
                "minecraft/versions".to_string(),
                minecraft_ttl(&env),
                || async {
                    service::minecraft_versions(&upstream)
                        .await
                        .map(|data| ApiResponse::success(data, now_iso()))
                },
            )
            .await
        }
        ["v1", "loaders", minecraft] => {
            if let Err(error) = validate_request_query(&req, &[]) {
                return json_error(&error, 400);
            }
            if let Err(error) = route_minecraft(minecraft) {
                return json_error(&error, 400);
            }
            let key = format!("loaders/{minecraft}");
            let upstream = upstream.clone();
            cached(
                req,
                &env,
                &ctx,
                key,
                ttl(&env, "CACHE_TTL_LOADERS", DEFAULT_CACHE_TTL_LOADERS),
                || async move {
                    Ok(ApiResponse::success(
                        service::loaders_for_minecraft(minecraft, &upstream).await,
                        now_iso(),
                    ))
                },
            )
            .await
        }
        ["v1", "dependencies", minecraft] => {
            if let Err(error) = route_minecraft(minecraft) {
                return json_error(&error, 400);
            }
            let query = request_query(&req)?;
            if let Err(error) = validate_query_keys(&query, &["mods"]) {
                return json_error(&error, 400);
            }
            let mods = normalize_list(query.get("mods").map(String::as_str), DEFAULT_MODS);
            if let Err(error) = validate_mods(&mods) {
                return json_error(&error, 400);
            }
            let key = dependency_cache_key(minecraft, &mods);
            let upstream = upstream.clone();
            cached(
                req,
                &env,
                &ctx,
                key,
                ttl(
                    &env,
                    "CACHE_TTL_DEPENDENCIES",
                    DEFAULT_CACHE_TTL_DEPENDENCIES,
                ),
                || async move {
                    let data =
                        service::dependencies_for_minecraft(minecraft, &mods, &upstream).await;
                    Ok(ApiResponse::success(data, now_iso()))
                },
            )
            .await
        }
        ["v1", "mods", "compatibility"] => {
            let query = request_query(&req)?;
            if let Err(error) = validate_query_keys(&query, &["minecraft", "mods"]) {
                return json_error(&error, 400);
            }
            let Some(mods_raw) = query.get("mods") else {
                return json_error("mods query parameter is required", 400);
            };
            let Some(minecraft_raw) = query.get("minecraft") else {
                return json_error("minecraft query parameter is required", 400);
            };

            let mods = normalize_list(Some(mods_raw), &[]);
            let minecraft_versions = normalize_list(Some(minecraft_raw), &[]);
            if mods.is_empty() || minecraft_versions.is_empty() {
                return json_error("mods and minecraft query parameters must not be empty", 400);
            }
            if let Err(error) = validate_mods(&mods) {
                return json_error(&error, 400);
            }
            if let Err(error) = validate_compatibility_minecraft_versions(&minecraft_versions) {
                return json_error(&error, 400);
            }

            let key = compatibility_cache_key(&mods, &minecraft_versions);
            let upstream = upstream.clone();
            cached(
                req,
                &env,
                &ctx,
                key,
                ttl(
                    &env,
                    "CACHE_TTL_COMPATIBILITY",
                    DEFAULT_CACHE_TTL_COMPATIBILITY,
                ),
                || async move {
                    let data = service::compatibility(&mods, &minecraft_versions, &upstream).await;
                    Ok(ApiResponse::success(data, now_iso()))
                },
            )
            .await
        }
        _ => json_error("not found", 404),
    }?;

    Ok(with_cors(response)?)
}

#[cfg(target_arch = "wasm32")]
async fn enforce_rate_limits(req: &Request, env: &Env) -> Result<Option<Response>> {
    let client = client_rate_limit_id(req)?;
    let public_key = rate_limit_key(&client, "api");
    let public_limiter = env.rate_limiter(PUBLIC_API_RATE_LIMIT_BINDING)?;
    if !public_limiter.limit(public_key).await?.success {
        return Ok(Some(rate_limited_response(&format!(
            "rate limit exceeded: {PUBLIC_API_RATE_LIMIT_PER_MINUTE} requests per minute"
        ))?));
    }

    if req.headers().get(REFRESH_HEADER)?.as_deref() == Some("1") {
        let refresh_key = rate_limit_key(&client, "refresh");
        let refresh_limiter = env.rate_limiter(REFRESH_RATE_LIMIT_BINDING)?;
        if !refresh_limiter.limit(refresh_key).await?.success {
            return Ok(Some(rate_limited_response(&format!(
                "rate limit exceeded: {REFRESH_RATE_LIMIT_PER_MINUTE} refreshes per minute"
            ))?));
        }
    }

    Ok(None)
}

#[cfg(target_arch = "wasm32")]
fn client_rate_limit_id(req: &Request) -> Result<String> {
    Ok(req
        .headers()
        .get("CF-Connecting-IP")?
        .or_else(|| req.headers().get("X-Forwarded-For").ok().flatten())
        .and_then(|value| value.split(',').next().map(str::trim).map(str::to_string))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "local".to_string()))
}

#[cfg(target_arch = "wasm32")]
fn rate_limited_response(message: &str) -> Result<Response> {
    let payload: ApiResponse<serde_json::Value> = ApiResponse::error(message, now_iso());
    let mut response = Response::from_json(&payload)?.with_status(429);
    response
        .headers_mut()
        .set("Content-Type", "application/json; charset=utf-8")?;
    response.headers_mut().set("Retry-After", "60")?;
    with_cors(response)
}

#[cfg(target_arch = "wasm32")]
fn upstream_config(env: &Env) -> service::UpstreamConfig {
    service::UpstreamConfig {
        modrinth_token: env
            .secret(MODRINTH_TOKEN_SECRET)
            .ok()
            .map(|secret| secret.to_string()),
    }
}

#[cfg(target_arch = "wasm32")]
fn route_minecraft(minecraft: &str) -> std::result::Result<(), String> {
    validate_minecraft(minecraft)
}

#[cfg(target_arch = "wasm32")]
fn request_query(req: &Request) -> Result<BTreeMap<String, String>> {
    Ok(req
        .url()?
        .query_pairs()
        .into_owned()
        .collect::<BTreeMap<_, _>>())
}

#[cfg(target_arch = "wasm32")]
fn validate_request_query(req: &Request, allowed: &[&str]) -> std::result::Result<(), String> {
    let query = req
        .url()
        .map_err(|error| error.to_string())?
        .query_pairs()
        .into_owned()
        .collect::<BTreeMap<_, _>>();
    validate_query_keys(&query, allowed)
}

#[cfg(target_arch = "wasm32")]
async fn health(upstream_config: &service::UpstreamConfig) -> Result<Response> {
    let checks = [
        ("mojang", MOJANG_MANIFEST_URL),
        ("fabric", FABRIC_LOADER_BASE_URL),
        ("forge", FORGE_METADATA_URL),
        ("neoforge", NEOFORGE_METADATA_URL),
        ("modrinth", "https://api.modrinth.com/v2/tag/game_version"),
        ("parchment", PARCHMENT_HEALTH_URL),
        ("maven_plugins", FORGEGRADLE_METADATA_URL),
    ];

    let futures = checks.iter().map(|(name, url)| async move {
        (
            (*name).to_string(),
            service::fetch_health(url, upstream_config).await,
        )
    });
    let upstream = futures_util::future::join_all(futures)
        .await
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    let status = if upstream
        .values()
        .filter(|value| value.as_str() == "error")
        .count()
        > 3
    {
        "degraded"
    } else {
        "ok"
    };
    json_ok(ApiResponse::success(
        HealthData {
            status: status.to_string(),
            upstream,
        },
        now_iso(),
    ))
}

#[cfg(target_arch = "wasm32")]
async fn cached<T, F, Fut>(
    req: Request,
    _env: &Env,
    ctx: &Context,
    key: String,
    ttl_seconds: u64,
    build: F,
) -> Result<Response>
where
    T: serde::Serialize,
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<ApiResponse<T>, String>>,
{
    let bypass = req.headers().get(REFRESH_HEADER)?.as_deref() == Some("1");
    let cache_url = format!("{CACHE_BASE_URL}/{RESPONSE_CACHE_VERSION}/{key}");
    let cache = Cache::default();

    if !bypass {
        if let Some(mut response) = cache.get(cache_url.as_str(), false).await? {
            let body = response.text().await?;
            let mut replay = Response::ok(body)?;
            replay
                .headers_mut()
                .set("Content-Type", "application/json; charset=utf-8")?;
            set_cache_headers(&mut replay, ttl_seconds)?;
            return Ok(replay);
        }
    }

    let payload = match build().await {
        Ok(payload) => payload,
        Err(error) => return json_error(&error, 500),
    };
    let cacheable = payload_is_cacheable(&payload);
    let mut response = json_ok(payload)?;
    set_cache_headers(&mut response, ttl_seconds)?;

    if response.status_code() == 200 && cacheable {
        let mut cache_response = response.cloned()?;
        set_cache_headers(&mut cache_response, ttl_seconds)?;
        ctx.wait_until(async move {
            let _ = cache.put(cache_url, cache_response).await;
        });
    }

    Ok(response)
}

fn payload_is_cacheable<T: serde::Serialize>(payload: &crate::types::ApiResponse<T>) -> bool {
    if !payload.success {
        return false;
    }

    serde_json::to_value(payload)
        .map(|value| !contains_error_status(&value))
        .unwrap_or(false)
}

fn contains_error_status(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Array(values) => values.iter().any(contains_error_status),
        serde_json::Value::Object(fields) => {
            fields.get("status").and_then(serde_json::Value::as_str) == Some("error")
                || fields.values().any(contains_error_status)
        }
        _ => false,
    }
}

#[cfg(target_arch = "wasm32")]
fn ttl(env: &Env, name: &str, default_value: u64) -> u64 {
    env.var(name)
        .ok()
        .and_then(|value| value.to_string().parse::<u64>().ok())
        .unwrap_or(default_value)
}

#[cfg(target_arch = "wasm32")]
fn minecraft_ttl(env: &Env) -> u64 {
    let normal_ttl = ttl(env, "CACHE_TTL_MINECRAFT", DEFAULT_CACHE_TTL_MINECRAFT);
    minecraft_manifest_ttl_for_utc_day(current_utc_day(), normal_ttl)
}

#[cfg(target_arch = "wasm32")]
fn json_ok<T: serde::Serialize>(payload: ApiResponse<T>) -> Result<Response> {
    let mut response = Response::from_json(&payload)?;
    response
        .headers_mut()
        .set("Content-Type", "application/json; charset=utf-8")?;
    Ok(response)
}

#[cfg(target_arch = "wasm32")]
fn json_error(message: &str, status: u16) -> Result<Response> {
    let payload: ApiResponse<serde_json::Value> = ApiResponse::error(message, now_iso());
    let mut response = Response::from_json(&payload)?.with_status(status);
    response
        .headers_mut()
        .set("Content-Type", "application/json; charset=utf-8")?;
    with_cors(response)
}

#[cfg(target_arch = "wasm32")]
fn method_not_allowed_response() -> Result<Response> {
    let mut response = json_error("method not allowed", 405)?;
    response.headers_mut().set("Allow", "GET, OPTIONS")?;
    Ok(response)
}

#[cfg(target_arch = "wasm32")]
fn set_cache_headers(response: &mut Response, ttl_seconds: u64) -> Result<()> {
    response.headers_mut().set(
        "Cache-Control",
        &format!("public, max-age={ttl_seconds}, stale-while-revalidate={ttl_seconds}"),
    )
}

#[cfg(target_arch = "wasm32")]
fn with_cors(mut response: Response) -> Result<Response> {
    response
        .headers_mut()
        .set("Access-Control-Allow-Origin", "*")?;
    response
        .headers_mut()
        .set("Access-Control-Allow-Methods", "GET, OPTIONS")?;
    response.headers_mut().set(
        "Access-Control-Allow-Headers",
        "Content-Type, X-Launcher-Meta-Refresh",
    )?;
    Ok(response)
}

#[cfg(target_arch = "wasm32")]
fn options_response() -> Result<Response> {
    with_cors(Response::empty()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ApiResponse, ItemStatus, LoaderItem, LoadersData};
    use serde_json::json;

    #[test]
    fn cacheable_payload_rejects_item_level_errors() {
        let payload = ApiResponse::success(
            json!({
                "dependencies": [
                    {
                        "id": "loom",
                        "status": "error",
                        "error": "TimeoutError"
                    }
                ]
            }),
            "2026-05-28T00:00:00.000Z".to_string(),
        );

        assert!(!payload_is_cacheable(&payload));
    }

    #[test]
    fn cacheable_payload_allows_unavailable_items() {
        let payload = ApiResponse::success(
            json!({
                "dependencies": [
                    {
                        "id": "rei",
                        "status": "unavailable"
                    }
                ]
            }),
            "2026-05-28T00:00:00.000Z".to_string(),
        );

        assert!(payload_is_cacheable(&payload));
    }

    #[test]
    fn cacheable_payload_rejects_route_errors() {
        let payload = ApiResponse::<serde_json::Value>::error(
            "upstream aggregation failure",
            "2026-05-28T00:00:00.000Z".to_string(),
        );

        assert!(!payload_is_cacheable(&payload));
    }

    #[test]
    fn cacheable_payload_rejects_loader_item_errors() {
        let payload = ApiResponse::success(
            LoadersData {
                minecraft: "26.1.2".to_string(),
                loaders: vec![LoaderItem {
                    loader: "forge".to_string(),
                    status: ItemStatus::Error,
                    version: None,
                    maven: None,
                    source: "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json"
                        .to_string(),
                    error: Some("TimeoutError".to_string()),
                }],
            },
            "2026-05-28T00:00:00.000Z".to_string(),
        );

        assert!(!payload_is_cacheable(&payload));
    }

    #[test]
    fn cacheable_payload_allows_unavailable_loader_items() {
        let payload = ApiResponse::success(
            LoadersData {
                minecraft: "26.1.2".to_string(),
                loaders: vec![LoaderItem {
                    loader: "forge".to_string(),
                    status: ItemStatus::Unavailable,
                    version: None,
                    maven: None,
                    source: "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json"
                        .to_string(),
                    error: None,
                }],
            },
            "2026-05-28T00:00:00.000Z".to_string(),
        );

        assert!(payload_is_cacheable(&payload));
    }
}
