pub const RESPONSE_CACHE_VERSION: &str = "v1";
pub const CACHE_BASE_URL: &str = "https://launcher-meta-cache.invalid";
pub const REFRESH_HEADER: &str = "X-Launcher-Meta-Refresh";
pub const PUBLIC_API_RATE_LIMIT_BINDING: &str = "PUBLIC_API_RATE_LIMIT";
pub const REFRESH_RATE_LIMIT_BINDING: &str = "REFRESH_RATE_LIMIT";
pub const PUBLIC_API_RATE_LIMIT_PER_MINUTE: u64 = 300;
pub const REFRESH_RATE_LIMIT_PER_MINUTE: u64 = 30;
pub const USER_AGENT: &str = "launcher-meta/0.1.0 (https://github.com/iamkaf/launcher-meta)";
pub const MODRINTH_TOKEN_SECRET: &str = "MODRINTH_TOKEN";
pub const UPSTREAM_TIMEOUT_MS: u32 = 10_000;

pub const DEFAULT_CACHE_TTL_MINECRAFT: u64 = 6 * 60 * 60;
pub const MINECRAFT_TUESDAY_TTL: u64 = 5 * 60;
pub const MINECRAFT_WEDNESDAY_TTL: u64 = 10 * 60;
pub const MINECRAFT_THURSDAY_TTL: u64 = 30 * 60;
pub const DEFAULT_CACHE_TTL_LOADERS: u64 = 30 * 60;
pub const DEFAULT_CACHE_TTL_DEPENDENCIES: u64 = 30 * 60;
pub const DEFAULT_CACHE_TTL_COMPATIBILITY: u64 = 30 * 60;

pub const MOJANG_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const FABRIC_LOADER_BASE_URL: &str = "https://meta.fabricmc.net/v2/versions/loader";
pub const FORGE_METADATA_URL: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json";
pub const NEOFORGE_METADATA_URL: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";
pub const NEOFORGE_LEGACY_METADATA_URL: &str =
    "https://maven.neoforged.net/releases/net/neoforged/forge/maven-metadata.xml";
pub const MODRINTH_PROJECT_BASE_URL: &str = "https://api.modrinth.com/v2/project";
pub const PARCHMENT_REPOSITORY_URL: &str =
    "https://ldtteam.jfrog.io/artifactory/parchmentmc-public/";
pub const PARCHMENT_BASE_URL: &str = "https://ldtteam.jfrog.io/artifactory/parchmentmc-public/org/parchmentmc/data/parchment-{version}/maven-metadata.xml";
pub const NEOFORM_METADATA_URL: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoform/maven-metadata.xml";
pub const FORGEGRADLE_METADATA_URL: &str = "https://plugins.gradle.org/m2/net/minecraftforge/gradle/net.minecraftforge.gradle.gradle.plugin/maven-metadata.xml";
pub const MODDEV_GRADLE_METADATA_URL: &str =
    "https://maven.neoforged.net/releases/net/neoforged/moddev-gradle/maven-metadata.xml";
pub const LOOM_METADATA_URL: &str =
    "https://maven.fabricmc.net/net/fabricmc/fabric-loom/maven-metadata.xml";

pub const BUILT_INS: &[&str] = &[
    "forge",
    "neoforge",
    "fabric-loader",
    "parchment",
    "neoform",
    "forgegradle",
    "moddev-gradle",
    "loom",
];

pub const DEFAULT_PROJECTS: &[&str] = &[
    "amber",
    "fabric-api",
    "modmenu",
    "rei",
    "architectury-api",
    "forge-config-api-port",
];
