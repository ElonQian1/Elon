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
    let config_path = cache_root.join("config").join("sccache-config");
    let cache_dir = cache_root.join("sccache");
    let args = env::args_os().skip(1);

    let status = Command::new(SCCACHE_PATH)
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
