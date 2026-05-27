use serde_json::{Value, json};

pub fn api_docs() -> Value {
    json!({
        "name": "launcher-meta",
        "version": "v1",
        "base_url": "https://launcher-meta.kaf.sh/v1",
        "description": "Current Minecraft, loader, and mod metadata behind a small JSON API.",
        "response_envelope": {
            "success": "boolean",
            "data": "object, present on success",
            "error": "string, present on failure",
            "timestamp": "ISO-8601 UTC timestamp",
            "cached_at": "ISO-8601 UTC timestamp, present when the response was generated from fresh upstream data"
        },
        "headers": {
            "User-Agent": {
                "value": "launcher-meta/0.1.0 (https://github.com/iamkaf/launcher-meta)",
                "description": "Sent on all upstream metadata requests."
            },
            "X-Launcher-Meta-Refresh": {
                "value": "1",
                "description": "Bypasses the edge response cache and repopulates it with a fresh successful response."
            }
        },
        "optional_secrets": {
            "MODRINTH_TOKEN": {
                "description": "Optional Modrinth API token. When present, Modrinth API requests include Authorization: Bearer <token>.",
                "local_development": "Set MODRINTH_TOKEN in .dev.vars or .env.",
                "production": "Set with `npx wrangler secret put MODRINTH_TOKEN`."
            }
        },
        "license": "Apache-2.0",
        "disclaimers": [
            "launcher-meta is not affiliated with Mojang, Microsoft, FabricMC, Minecraft Forge, NeoForge, Modrinth, ParchmentMC, or Cloudflare.",
            "Minecraft names, loader names, Modrinth mod names, and service names belong to their respective owners.",
            "The API reports metadata from upstream services and caches successful responses at the edge. Treat versions as metadata snapshots, not endorsements or compatibility guarantees."
        ],
        "acknowledgements": [
            "Mojang version manifest",
            "FabricMC metadata and Maven",
            "Minecraft Forge metadata",
            "NeoForge metadata and Maven",
            "Modrinth API",
            "ParchmentMC Maven",
            "Gradle Plugin Portal",
            "Cloudflare Workers"
        ],
        "cache": {
            "minecraft_versions": {
                "normal": "21600 seconds",
                "tuesday_utc": "300 seconds",
                "wednesday_utc": "600 seconds",
                "thursday_utc": "1800 seconds"
            },
            "loaders": "1800 seconds",
            "dependencies": "1800 seconds",
            "compatibility": "1800 seconds"
        },
        "rate_limits": {
            "scope": "per client key per Cloudflare location",
            "client_key": "CF-Connecting-IP, with X-Forwarded-For/local fallback",
            "general_api": {
                "limit": 300,
                "period": "60 seconds",
                "description": "Applies to all non-OPTIONS /v1 requests."
            },
            "refresh": {
                "limit": 30,
                "period": "60 seconds",
                "description": "Applies in addition to the general API limit when X-Launcher-Meta-Refresh: 1 is sent."
            },
            "response": {
                "status": 429,
                "retry_after": "60 seconds",
                "body": {
                    "success": false,
                    "error": "rate limit exceeded",
                    "timestamp": "ISO-8601 UTC timestamp"
                }
            }
        },
        "request_limits": {
            "mods": {
                "max_entries": 32,
                "max_id_length": 128,
                "allowed_characters": "ASCII letters, digits, hyphen, underscore"
            },
            "compatibility_minecraft_versions": {
                "max_entries": 16
            }
        },
        "endpoints": [
            {
                "method": "GET",
                "path": "/v1",
                "description": "Returns this API documentation.",
                "cache": "not cached",
                "example_response": {
                    "success": true,
                    "data": {
                        "name": "launcher-meta",
                        "version": "v1",
                        "base_url": "https://launcher-meta.kaf.sh/v1",
                        "license": "Apache-2.0",
                        "endpoints": [
                            {
                                "method": "GET",
                                "path": "/v1/minecraft/versions"
                            }
                        ]
                    },
                    "timestamp": "2026-05-27T12:00:00.000Z",
                    "cached_at": "2026-05-27T12:00:00.000Z"
                }
            },
            {
                "method": "GET",
                "path": "/v1/health",
                "description": "Checks worker status and upstream metadata services.",
                "cache": "not cached",
                "data_shape": {
                    "status": "ok | degraded",
                    "upstream": {
                        "mojang": "ok | error",
                        "fabric": "ok | error",
                        "forge": "ok | error",
                        "neoforge": "ok | error",
                        "modrinth": "ok | error",
                        "parchment": "ok | error",
                        "maven_plugins": "ok | error"
                    }
                },
                "example_response": {
                    "success": true,
                    "data": {
                        "status": "ok",
                        "upstream": {
                            "mojang": "ok",
                            "fabric": "ok",
                            "forge": "ok",
                            "neoforge": "ok",
                            "modrinth": "ok",
                            "parchment": "ok",
                            "maven_plugins": "ok"
                        }
                    },
                    "timestamp": "2026-05-27T12:00:00.000Z",
                    "cached_at": "2026-05-27T12:00:00.000Z"
                }
            },
            {
                "method": "GET",
                "path": "/v1/minecraft/versions",
                "description": "Returns all Minecraft versions from Mojang's version manifest.",
                "cache": "dynamic: 5m Tue UTC, 10m Wed UTC, 30m Thu UTC, 6h otherwise",
                "data_shape": {
                    "versions": [
                        {
                            "id": "1.21.4",
                            "kind": "release | snapshot | old_beta | old_alpha"
                        }
                    ]
                },
                "example_response": {
                    "success": true,
                    "data": {
                        "versions": [
                            { "id": "1.21.4", "kind": "release" },
                            { "id": "25w14craftmine", "kind": "snapshot" }
                        ]
                    },
                    "timestamp": "2026-05-27T12:00:00.000Z",
                    "cached_at": "2026-05-27T12:00:00.000Z"
                }
            },
            {
                "method": "GET",
                "path": "/v1/loaders/{minecraft}",
                "description": "Returns Fabric, Forge, and NeoForge loader metadata for one Minecraft version.",
                "path_parameters": {
                    "minecraft": "Minecraft version id, for example 1.21.4"
                },
                "cache": "1800 seconds",
                "data_shape": {
                    "minecraft": "1.21.4",
                    "loaders": [
                        {
                            "loader": "fabric | forge | neoforge",
                            "status": "ok | error | unavailable",
                            "version": "resolved loader version or null",
                            "maven": "Maven coordinate or null",
                            "source": "upstream metadata URL",
                            "error": "present when status is error"
                        }
                    ]
                },
                "example_response": {
                    "success": true,
                    "data": {
                        "minecraft": "1.21.4",
                        "loaders": [
                            {
                                "loader": "fabric",
                                "status": "ok",
                                "version": "0.16.14",
                                "maven": "net.fabricmc:fabric-loader:0.16.14",
                                "source": "https://meta.fabricmc.net/v2/versions/loader/1.21.4"
                            },
                            {
                                "loader": "forge",
                                "status": "ok",
                                "version": "1.21.4-54.0.0",
                                "maven": "net.minecraftforge:forge:1.21.4-54.0.0:installer",
                                "source": "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json"
                            },
                            {
                                "loader": "neoforge",
                                "status": "ok",
                                "version": "21.4.0-beta",
                                "maven": "net.neoforged:neoforge:21.4.0-beta:installer",
                                "source": "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml"
                            }
                        ]
                    },
                    "timestamp": "2026-05-27T12:00:00.000Z",
                    "cached_at": "2026-05-27T12:00:00.000Z"
                }
            },
            {
                "method": "GET",
                "path": "/v1/dependencies/{minecraft}",
                "description": "Returns built-in loader/build dependencies plus Modrinth mod metadata for one Minecraft version.",
                "path_parameters": {
                    "minecraft": "Minecraft version id, for example 1.21.4"
                },
                "query_parameters": {
                    "mods": {
                        "required": false,
                        "description": "Comma-separated Modrinth mod slugs. Replaces the default Modrinth mod list when supplied.",
                        "max_entries": 32,
                        "allowed_characters": "ASCII letters, digits, hyphen, underscore"
                    }
                },
                "default_mods": [
                    "amber",
                    "fabric-api",
                    "modmenu",
                    "rei",
                    "architectury-api",
                    "forge-config-api-port"
                ],
                "built_ins": [
                    "forge",
                    "neoforge",
                    "fabric-loader",
                    "parchment",
                    "neoform",
                    "forgegradle",
                    "moddev-gradle",
                    "loom"
                ],
                "cache": "1800 seconds",
                "failure_model": "Partial failures return HTTP 200 with per-item status=error.",
                "rules": {
                    "parchment": "Parchment is unavailable for unobfuscated Minecraft versions. Minecraft is treated as unobfuscated starting at 26.1."
                },
                "data_shape": {
                    "minecraft": "1.21.4",
                    "dependencies": [
                        {
                            "id": "fabric-api",
                            "kind": "loader | mapping | tool | mod",
                            "status": "ok | error | unavailable",
                            "version": "resolved version or null",
                            "loader_versions": {
                                "forge": "version or null",
                                "neoforge": "version or null",
                                "fabric": "version or null"
                            },
                            "coordinates": "Maven coordinate or null",
                            "source": "upstream source URL",
                            "error": "present when status is error"
                        }
                    ]
                },
                "example_response": {
                    "success": true,
                    "data": {
                        "minecraft": "1.21.4",
                        "dependencies": [
                            {
                                "id": "fabric-loader",
                                "kind": "loader",
                                "status": "ok",
                                "version": "0.16.14",
                                "loader_versions": {
                                    "forge": null,
                                    "neoforge": null,
                                    "fabric": "0.16.14"
                                },
                                "coordinates": "net.fabricmc:fabric-loader:0.16.14",
                                "source": "https://meta.fabricmc.net/v2/versions/loader/1.21.4"
                            },
                            {
                                "id": "fabric-api",
                                "kind": "mod",
                                "status": "ok",
                                "version": "0.110.5+1.21.4",
                                "loader_versions": {
                                    "forge": null,
                                    "neoforge": null,
                                    "fabric": "0.110.5+1.21.4"
                                },
                                "coordinates": "maven.modrinth:fabric-api:0.110.5+1.21.4",
                                "source": "https://modrinth.com/mod/fabric-api"
                            },
                            {
                                "id": "bad-mod",
                                "kind": "mod",
                                "status": "error",
                                "version": null,
                                "loader_versions": {
                                    "forge": null,
                                    "neoforge": null,
                                    "fabric": null
                                },
                                "coordinates": null,
                                "source": "https://modrinth.com/mod/bad-mod",
                                "error": "upstream returned HTTP 404"
                            }
                        ]
                    },
                    "timestamp": "2026-05-27T12:00:00.000Z",
                    "cached_at": "2026-05-27T12:00:00.000Z"
                }
            },
            {
                "method": "GET",
                "path": "/v1/mods/compatibility",
                "description": "Returns loader-specific Modrinth mod versions across multiple Minecraft versions.",
                "query_parameters": {
                    "mods": {
                        "required": true,
                        "description": "Comma-separated Modrinth mod slugs.",
                        "max_entries": 32,
                        "allowed_characters": "ASCII letters, digits, hyphen, underscore"
                    },
                    "minecraft": {
                        "required": true,
                        "description": "Comma-separated Minecraft version ids.",
                        "max_entries": 16
                    }
                },
                "cache": "1800 seconds",
                "failure_model": "Unresolved mod/version pairs return null loader fields.",
                "data_shape": {
                    "mods": {
                        "fabric-api": {
                            "1.21.4": {
                                "forge": null,
                                "neoforge": null,
                                "fabric": "0.110.5+1.21.4"
                            }
                        }
                    }
                },
                "example_response": {
                    "success": true,
                    "data": {
                        "mods": {
                            "fabric-api": {
                                "1.21.1": {
                                    "forge": null,
                                    "neoforge": null,
                                    "fabric": "0.116.7+1.21.1"
                                },
                                "1.21.4": {
                                    "forge": null,
                                    "neoforge": null,
                                    "fabric": "0.110.5+1.21.4"
                                }
                            }
                        }
                    },
                    "timestamp": "2026-05-27T12:00:00.000Z",
                    "cached_at": "2026-05-27T12:00:00.000Z"
                }
            }
        ],
        "errors": {
            "400": "Invalid or missing request parameters.",
            "405": "Method not allowed. Use GET or OPTIONS.",
            "429": "Rate limit exceeded.",
            "404": "Unknown route.",
            "500": "Unexpected worker or upstream aggregation failure."
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docs_include_all_public_endpoints() {
        let docs = api_docs();
        let endpoints = docs["endpoints"]
            .as_array()
            .expect("endpoints should be an array");
        let paths = endpoints
            .iter()
            .map(|endpoint| endpoint["path"].as_str().unwrap())
            .collect::<Vec<_>>();

        assert!(paths.contains(&"/v1"));
        assert!(paths.contains(&"/v1/health"));
        assert!(paths.contains(&"/v1/minecraft/versions"));
        assert!(paths.contains(&"/v1/loaders/{minecraft}"));
        assert!(paths.contains(&"/v1/dependencies/{minecraft}"));
        assert!(paths.contains(&"/v1/mods/compatibility"));
    }
}
