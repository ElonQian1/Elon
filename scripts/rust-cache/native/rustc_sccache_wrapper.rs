use std::{env, process::Command};

const SCCACHE_PATH: &str = env!("ELON_RUST_CACHE_SCCACHE_PATH");
const SCCACHE_CACHE_SIZE: &str = env!("ELON_RUST_CACHE_SCCACHE_SIZE");

fn run() -> Result<i32, String> {
    let executable = env::current_exe()
        .map_err(|error| format!("cannot locate native sccache wrapper: {error}"))?;
    let platform_dir = executable
        .parent()
        .ok_or_else(|| "native sccache wrapper has no platform directory".to_owned())?;
    let cache_root = platform_dir
        .parent()
        .ok_or_else(|| "native sccache wrapper has no cache root".to_owned())?;
    let config_path = option_env!("ELON_RUST_CACHE_SCCACHE_CONF")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| cache_root.join("config").join("sccache-config"));
    let cache_dir = option_env!("ELON_RUST_CACHE_SCCACHE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| cache_root.join("sccache"));
    let args = env::args_os().skip(1);

    let mut command = Command::new(SCCACHE_PATH);
    if let Some(port) = option_env!("ELON_RUST_CACHE_SCCACHE_PORT") {
        // Backend routing is generated from the machine's reviewed tier configuration.
        // Authentication remains in the standard provider credential environment.
        for name in [
            "SCCACHE_BASEDIRS",
            "SCCACHE_MULTILEVEL_CHAIN",
            "SCCACHE_MULTILEVEL_WRITE_ERROR_POLICY",
            "SCCACHE_LOCAL_RW_MODE",
            "SCCACHE_BUCKET",
            "SCCACHE_ENDPOINT",
            "SCCACHE_REGION",
            "SCCACHE_S3_USE_SSL",
            "SCCACHE_S3_KEY_PREFIX",
            "SCCACHE_S3_RW_MODE",
            "SCCACHE_S3_NO_CREDENTIALS",
            "SCCACHE_S3_ENABLE_VIRTUAL_HOST_STYLE",
            "SCCACHE_S3_SERVER_SIDE_ENCRYPTION",
            "SCCACHE_WEBDAV_ENDPOINT",
            "SCCACHE_WEBDAV_KEY_PREFIX",
            "SCCACHE_WEBDAV_RW_MODE",
            "SCCACHE_REDIS",
            "SCCACHE_REDIS_ENDPOINT",
            "SCCACHE_REDIS_CLUSTER_ENDPOINTS",
            "SCCACHE_MEMCACHED",
            "SCCACHE_MEMCACHED_ENDPOINT",
            "SCCACHE_GCS_BUCKET",
            "SCCACHE_AZURE_CONNECTION_STRING",
            "SCCACHE_AZURE_BLOB_CONTAINER",
            "SCCACHE_GHA_CACHE_URL",
            "SCCACHE_GHA_ENABLED",
            "SCCACHE_GHA_VERSION",
            "SCCACHE_GHA_RW_MODE",
            "SCCACHE_OSS_BUCKET",
            "SCCACHE_COS_BUCKET",
        ] {
            command.env_remove(name);
        }
        command.env("SCCACHE_SERVER_PORT", port);
        command.env("SCCACHE_IDLE_TIMEOUT", "0");
        command.env(
            "SCCACHE_CACHED_CONF",
            option_env!("ELON_RUST_CACHE_SCCACHE_CACHED_CONF").unwrap_or(""),
        );
        if let Some(endpoint) = option_env!("ELON_RUST_CACHE_SCCACHE_WEBDAV_ENDPOINT") {
            command.env("SCCACHE_WEBDAV_ENDPOINT", endpoint);
            command.env(
                "SCCACHE_WEBDAV_KEY_PREFIX",
                option_env!("ELON_RUST_CACHE_SCCACHE_WEBDAV_KEY_PREFIX").unwrap_or(""),
            );
            command.env("SCCACHE_WEBDAV_RW_MODE", "READ_WRITE");
        }
    }
    let status = command
        .args(args)
        .env("SCCACHE_CONF", config_path)
        .env("SCCACHE_DIR", cache_dir)
        .env("SCCACHE_CACHE_SIZE", SCCACHE_CACHE_SIZE)
        .env_remove("CARGO_BUILD_BUILD_DIR")
        .env_remove("CARGO_TARGET_DIR")
        .status()
        .map_err(|error| format!("cannot start sccache at {SCCACHE_PATH}: {error}"))?;
    Ok(status.code().unwrap_or(1))
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("rustc-sccache-wrapper: {error}");
            std::process::exit(1);
        }
    }
}
