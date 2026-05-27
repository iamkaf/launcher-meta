<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-a78bfa?style=for-the-badge&labelColor=0d1117" alt="Apache-2.0 License" /></a>
  <img src="https://img.shields.io/badge/rust-2024-5eead4?style=for-the-badge&logo=rust&logoColor=5eead4&labelColor=0d1117" alt="Rust 2024" />
  <img src="https://img.shields.io/badge/cloudflare-workers-fbbf24?style=for-the-badge&logo=cloudflare&logoColor=fbbf24&labelColor=0d1117" alt="Cloudflare Workers" />
</p>

<h1 align="center">launcher-meta</h1>

<p align="center">
  <strong>Current Minecraft, loader, and mod metadata behind a small JSON API.</strong>
</p>

<p align="center">
  <a href="#api">API</a> ·
  <a href="#cache-policy">Cache Policy</a> ·
  <a href="#rate-limits">Rate Limits</a> ·
  <a href="#maintainers">Maintainers</a>
</p>

---

`launcher-meta` gives launchers, build scripts, and release tooling one place to ask for current Minecraft versions, loader versions, build-tool versions, and Modrinth project compatibility.

The API runs at `https://launcher-meta.kaf.sh/v1` and returns JSON for every route.

## How It Works

```text
Ask for metadata
  -> get a typed JSON response
  -> check item-level status for partial failures
  -> refresh with a header when you need fresh data now
```

Batch endpoints keep returning useful data when one upstream project fails. Failed items carry `status: "error"` and an `error` field.

## Disclaimers

`launcher-meta` is not affiliated with Mojang, Microsoft, FabricMC, Minecraft Forge, NeoForge, Modrinth, ParchmentMC, or Cloudflare. Minecraft names, loader names, project names, and service names belong to their respective owners.

The API reports metadata from upstream services and caches successful responses at the edge. Treat versions as metadata snapshots, not as an endorsement or guarantee that a dependency is safe, compatible with every modpack, or available forever.

## API

All endpoints live under `/v1`.

Every response uses this envelope:

| Field | Type | Description |
| --- | --- | --- |
| `success` | boolean | Whether the request succeeded at the route level |
| `data` | object | Present on success |
| `error` | string | Present on route-level failure |
| `timestamp` | string | ISO-8601 UTC timestamp |
| `cached_at` | string | Present when the response was generated from fresh upstream data |

Use this header to bypass the edge cache and repopulate it:

```http
X-Launcher-Meta-Refresh: 1
```

### `GET /v1`

Returns the API documentation as structured JSON. This is the live docs endpoint.

```sh
curl https://launcher-meta.kaf.sh/v1
```

Example response:

```json
{
  "success": true,
  "data": {
    "name": "launcher-meta",
    "version": "v1",
    "base_url": "https://launcher-meta.kaf.sh/v1",
    "endpoints": [
      {
        "method": "GET",
        "path": "/v1/minecraft/versions",
        "description": "Returns all Minecraft versions from Mojang's version manifest."
      }
    ],
    "license": "Apache-2.0"
  },
  "timestamp": "2026-05-27T12:00:00.000Z",
  "cached_at": "2026-05-27T12:00:00.000Z"
}
```

### `GET /v1/health`

Checks the worker and upstream metadata sources.

| Response field | Description |
| --- | --- |
| `status` | `ok` or `degraded` |
| `upstream.mojang` | Mojang version manifest status |
| `upstream.fabric` | Fabric metadata status |
| `upstream.forge` | Forge metadata status |
| `upstream.neoforge` | NeoForge metadata status |
| `upstream.modrinth` | Modrinth API status |
| `upstream.parchment` | Parchment Maven status |
| `upstream.maven_plugins` | Gradle plugin metadata status |

```sh
curl https://launcher-meta.kaf.sh/v1/health
```

Example response:

```json
{
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
```

### `GET /v1/minecraft/versions`

Returns all Minecraft versions from Mojang's version manifest.

```json
{
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
```

```sh
curl https://launcher-meta.kaf.sh/v1/minecraft/versions
```

### `GET /v1/loaders/{minecraft}`

Returns Fabric, Forge, and NeoForge loader metadata for one Minecraft version.

| Field | Description |
| --- | --- |
| `loader` | `fabric`, `forge`, or `neoforge` |
| `status` | `ok`, `error`, or `unavailable` |
| `version` | Resolved loader version, or `null` |
| `maven` | Maven coordinate, or `null` |
| `source` | Upstream metadata URL |
| `error` | Present when `status` is `error` |

```sh
curl https://launcher-meta.kaf.sh/v1/loaders/1.21.4
```

Example response:

```json
{
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
```

### `GET /v1/dependencies/{minecraft}`

Returns built-in loader/build dependencies plus Modrinth project metadata for one Minecraft version.

| Query | Required | Description |
| --- | --- | --- |
| `projects` | No | Comma-separated Modrinth project slugs. Replaces the default project list when supplied |

Default Modrinth projects:

| Project |
| --- |
| `amber` |
| `fabric-api` |
| `modmenu` |
| `rei` |
| `architectury-api` |
| `forge-config-api-port` |

Built-ins are always included:

| Built-in |
| --- |
| `forge` |
| `neoforge` |
| `fabric-loader` |
| `parchment` |
| `neoform` |
| `forgegradle` |
| `moddev-gradle` |
| `loom` |

Dependency items include:

| Field | Description |
| --- | --- |
| `id` | Dependency or project id |
| `kind` | `loader`, `mapping`, `tool`, or `mod` |
| `status` | `ok`, `error`, or `unavailable` |
| `version` | Resolved version, or `null` |
| `loader_versions` | Loader-specific project versions |
| `coordinates` | Maven coordinate, or `null` |
| `source` | Upstream source URL |
| `error` | Present when `status` is `error` |

```sh
curl "https://launcher-meta.kaf.sh/v1/dependencies/1.21.4"
curl "https://launcher-meta.kaf.sh/v1/dependencies/1.21.4?projects=fabric-api,modmenu,sodium"
```

Example response:

```json
{
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
        "id": "bad-project",
        "kind": "mod",
        "status": "error",
        "version": null,
        "loader_versions": {
          "forge": null,
          "neoforge": null,
          "fabric": null
        },
        "coordinates": null,
        "source": "https://modrinth.com/mod/bad-project",
        "error": "upstream returned HTTP 404"
      }
    ]
  },
  "timestamp": "2026-05-27T12:00:00.000Z",
  "cached_at": "2026-05-27T12:00:00.000Z"
}
```

### `GET /v1/projects/compatibility`

Returns loader-specific project versions across multiple Minecraft versions.

| Query | Required | Description |
| --- | --- | --- |
| `projects` | Yes | Comma-separated Modrinth or built-in project ids |
| `minecraft` | Yes | Comma-separated Minecraft version ids |

Unresolved project/version pairs return `null` loader fields.

```sh
curl "https://launcher-meta.kaf.sh/v1/projects/compatibility?projects=fabric-api,modmenu&minecraft=1.21.1,1.21.4"
```

Example response:

```json
{
  "success": true,
  "data": {
    "projects": {
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
```

## Cache Policy

| Route | TTL |
| --- | --- |
| `/v1` | Not cached |
| `/v1/health` | Not cached |
| `/v1/minecraft/versions` | Dynamic |
| `/v1/loaders/{minecraft}` | 30 minutes |
| `/v1/dependencies/{minecraft}` | 30 minutes |
| `/v1/projects/compatibility` | 30 minutes |

Minecraft version data uses a tighter release-window policy:

| UTC day | TTL |
| --- | --- |
| Tuesday | 5 minutes |
| Wednesday | 10 minutes |
| Thursday | 30 minutes |
| Other days | 6 hours |

## Errors

| Status | Meaning |
| --- | --- |
| `400` | Invalid or missing request parameters |
| `405` | Method not allowed; use `GET` or `OPTIONS` |
| `429` | Rate limit exceeded |
| `404` | Unknown route |
| `500` | Unexpected worker or upstream aggregation failure |

Batch endpoints prefer partial results over route-level failure. Check item-level `status` and `error` fields before using a dependency version.

## Rate Limits

Rate limits are enforced per client key per Cloudflare location.

| Limit | Window | Applies to |
| --- | --- | --- |
| 300 requests | 60 seconds | All non-OPTIONS `/v1` requests |
| 30 refreshes | 60 seconds | Requests with `X-Launcher-Meta-Refresh: 1`, in addition to the general limit |

The client key uses `CF-Connecting-IP`, with a fallback for local development. A limited request returns HTTP `429`, `Retry-After: 60`, and the standard error envelope.

## Maintainers

`launcher-meta` runs as a Cloudflare Worker and uses Cloudflare's Cache API for successful response bodies.

All upstream requests send this User-Agent:

```http
User-Agent: launcher-meta/0.1.0 (https://github.com/iamkaf/launcher-meta)
```

### Configuration

| Variable | Default | Description |
| --- | --- | --- |
| `CACHE_TTL_MINECRAFT` | `21600` | Normal-day Minecraft manifest TTL in seconds |
| `CACHE_TTL_LOADERS` | `1800` | Loader metadata TTL in seconds |
| `CACHE_TTL_DEPENDENCIES` | `1800` | Dependency response TTL in seconds |
| `CACHE_TTL_COMPATIBILITY` | `1800` | Compatibility response TTL in seconds |

Rate limit bindings:

| Binding | Namespace | Limit |
| --- | --- | --- |
| `PUBLIC_API_RATE_LIMIT` | `7001001` | 300 requests per 60 seconds |
| `REFRESH_RATE_LIMIT` | `7001002` | 30 refreshes per 60 seconds |

Optional secret:

| Secret | Required | Description |
| --- | --- | --- |
| `MODRINTH_TOKEN` | No | Adds `Authorization: Bearer <token>` to Modrinth API requests |

### Development

Install the Rust Wasm target and `worker-build` once:

```sh
rustup target add wasm32-unknown-unknown
cargo install worker-build --version 0.8.3
```

Run checks:

```sh
cargo fmt --check
cargo test
cargo check --target wasm32-unknown-unknown
npx wrangler deploy --dry-run
```

Run locally:

```sh
npx wrangler dev
```

Deploy:

```sh
npx wrangler deploy
```

Set the production secret with Wrangler:

```sh
npx wrangler secret put MODRINTH_TOKEN
```

For local development, put it in `.dev.vars` or `.env`:

```dotenv
MODRINTH_TOKEN="..."
```

## Acknowledgements

`launcher-meta` depends on public metadata and APIs from Mojang, FabricMC, Minecraft Forge, NeoForge, Modrinth, ParchmentMC, Fabric Maven, NeoForged Maven, and the Gradle Plugin Portal.

## License

Apache-2.0. See [LICENSE](LICENSE).
