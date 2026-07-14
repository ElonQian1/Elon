use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

const GRADLE_WRAPPER_JAR: &[u8] =
    include_bytes!("../../../android/gradle/wrapper/gradle-wrapper.jar");
const GRADLEW_BAT: &str = include_str!("../../../android/gradlew.bat");
const GRADLEW_SH: &str = r#"#!/bin/sh
APP_HOME=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec java -classpath "$APP_HOME/gradle/wrapper/gradle-wrapper.jar" org.gradle.wrapper.GradleWrapperMain "$@"
"#;

pub(crate) fn pc_apk_sync_script(
    fresh_after_unix_secs: Option<u64>,
    build_if_missing: bool,
) -> String {
    let freshness_filter = fresh_after_unix_secs
        .map(|secs| {
            format!(
                "$minModifiedUtc = [DateTimeOffset]::FromUnixTimeSeconds({secs}).UtcDateTime\n$files = @($files | Where-Object {{ $_.LastWriteTimeUtc -ge $minModifiedUtc }})"
            )
        })
        .unwrap_or_default();
    let build_flag = if build_if_missing { "$true" } else { "$false" };

    SCRIPT_TEMPLATE
        .replace("__ELON_BUILD_IF_MISSING__", build_flag)
        .replace("__ELON_FRESHNESS_FILTER__", &freshness_filter)
        .replace("__ELON_WRAPPER_JAR_B64__", &B64.encode(GRADLE_WRAPPER_JAR))
        .replace(
            "__ELON_GRADLEW_BAT_B64__",
            &B64.encode(GRADLEW_BAT.as_bytes()),
        )
        .replace(
            "__ELON_GRADLEW_SH_B64__",
            &B64.encode(GRADLEW_SH.as_bytes()),
        )
}

pub(crate) fn pc_apk_sync_loader_command(
    public_url: &str,
    fresh_after_unix_secs: Option<u64>,
    build_if_missing: bool,
) -> String {
    let mut url = format!(
        "{}/api/agent/scripts/pc-apk-sync.ps1?build_if_missing={}",
        public_url.trim_end_matches('/'),
        if build_if_missing { "true" } else { "false" }
    );
    if let Some(secs) = fresh_after_unix_secs {
        url.push_str("&fresh_after_unix_secs=");
        url.push_str(&secs.to_string());
    }
    let url = powershell_single_quoted(&url);
    format!(
        "$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[System.Text.Encoding]::UTF8; $u='{url}'; $s=(Invoke-WebRequest -UseBasicParsing -Uri $u).Content; Invoke-Expression $s"
    )
}

#[path = "ai_cli_apk_build_script_impl.rs"]
mod impl_mod;
use self::impl_mod::*;
