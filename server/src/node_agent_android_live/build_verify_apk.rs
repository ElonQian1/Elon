use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

const MAX_SCAN_FILES: usize = 40_000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OutputMetadata {
    application_id: Option<String>,
    variant_name: Option<String>,
    #[serde(default)]
    elements: Vec<OutputElement>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OutputElement {
    output_file: String,
}

pub(crate) fn select_fresh_debug_apk(
    gradle_root: &Path,
    expected_application_id: &str,
    not_before: SystemTime,
) -> Result<PathBuf> {
    select_debug_apk(gradle_root, expected_application_id, Some(not_before))?.ok_or_else(|| {
        anyhow!(
            "本次构建未产生 applicationId={} 的新 Debug APK",
            expected_application_id
        )
    })
}

pub(crate) fn select_reusable_debug_apk(
    gradle_root: &Path,
    expected_application_id: &str,
) -> Result<Option<PathBuf>> {
    let Some(apk) = select_debug_apk(gradle_root, expected_application_id, None)? else {
        return Ok(None);
    };
    if artifact_is_current(gradle_root, &apk)? {
        Ok(Some(apk))
    } else {
        Ok(None)
    }
}

/// Selects the exact Debug APK produced or validated by a Gradle invocation that
/// has just completed successfully in `gradle_root`.
///
/// Gradle may restore an output from its build cache while preserving the cached
/// file timestamp. In that case the timestamp-only reusable-artifact check above
/// intentionally rejects the APK before a build, but the successful Gradle build
/// itself is the source-input proof. The metadata/application-id checks remain
/// mandatory so an unrelated or split APK can never be selected.
pub(crate) fn select_debug_apk_after_successful_build(
    gradle_root: &Path,
    expected_application_id: &str,
) -> Result<PathBuf> {
    select_debug_apk(gradle_root, expected_application_id, None)?.ok_or_else(|| {
        anyhow!(
            "Gradle 构建成功，但未找到 applicationId={} 的 Debug APK",
            expected_application_id
        )
    })
}

fn select_debug_apk(
    gradle_root: &Path,
    expected_application_id: &str,
    not_before: Option<SystemTime>,
) -> Result<Option<PathBuf>> {
    let canonical_root = gradle_root
        .canonicalize()
        .context("Gradle 根目录不可访问")?;
    let mut metadata_files = Vec::new();
    let mut visited = 0;
    collect_metadata(&canonical_root, 0, &mut visited, &mut metadata_files)?;
    let mut seen_application_ids = BTreeSet::new();
    let mut candidates = BTreeSet::new();
    for metadata_path in metadata_files {
        let bytes = fs::read(&metadata_path).with_context(|| {
            format!("读取 APK output metadata 失败: {}", metadata_path.display())
        })?;
        let metadata: OutputMetadata = serde_json::from_slice(&bytes)
            .with_context(|| format!("APK output metadata 无效: {}", metadata_path.display()))?;
        let Some(application_id) = metadata.application_id else {
            continue;
        };
        seen_application_ids.insert(application_id.clone());
        if application_id != expected_application_id
            || !metadata
                .variant_name
                .as_deref()
                .is_some_and(|value| value.to_ascii_lowercase().contains("debug"))
        {
            continue;
        }
        let parent = metadata_path
            .parent()
            .ok_or_else(|| anyhow!("output-metadata.json 缺少父目录"))?;
        for element in metadata.elements {
            if element.output_file.trim().is_empty() {
                continue;
            }
            let candidate = parent.join(&element.output_file);
            let canonical = candidate
                .canonicalize()
                .with_context(|| format!("metadata 引用的 APK 不存在: {}", candidate.display()))?;
            if !canonical.starts_with(&canonical_root)
                || canonical.extension().and_then(|value| value.to_str()) != Some("apk")
            {
                bail!(
                    "output metadata 引用了项目外或非 APK 文件: {}",
                    canonical.display()
                );
            }
            let modified = fs::metadata(&canonical)?.modified()?;
            if not_before.is_some_and(|not_before| modified < not_before) {
                continue;
            }
            candidates.insert(canonical);
        }
    }
    match candidates.len() {
        1 => Ok(Some(
            candidates
                .into_iter()
                .next()
                .expect("candidate count checked"),
        )),
        0 if not_before.is_some() => bail!(
            "本次构建未产生 applicationId={} 的新 Debug APK；metadata 中发现: {:?}",
            expected_application_id,
            seen_application_ids
        ),
        0 => Ok(None),
        count => bail!(
            "applicationId={} 对应 {count} 个新 Debug APK，无法安全选择（可能是 split APK）",
            expected_application_id
        ),
    }
}

fn artifact_is_current(gradle_root: &Path, apk: &Path) -> Result<bool> {
    let apk_modified = fs::metadata(apk)?.modified()?;
    let canonical_root = gradle_root
        .canonicalize()
        .context("Gradle 根目录不可访问")?;
    let mut visited = 0;
    source_inputs_not_newer(&canonical_root, 0, &mut visited, apk_modified)
}

fn source_inputs_not_newer(
    dir: &Path,
    depth: usize,
    visited: &mut usize,
    apk_modified: SystemTime,
) -> Result<bool> {
    if depth > 14 || *visited > MAX_SCAN_FILES {
        return Ok(false);
    }
    for entry in
        fs::read_dir(dir).with_context(|| format!("读取源码目录失败: {}", dir.display()))?
    {
        let entry = entry?;
        *visited += 1;
        let path = entry.path();
        if path.is_dir() {
            if !matches!(
                entry.file_name().to_str(),
                Some(".git" | ".gradle" | "build" | "node_modules" | "target")
            ) && !source_inputs_not_newer(&path, depth + 1, visited, apk_modified)?
            {
                return Ok(false);
            }
        } else if is_build_input(&path) && fs::metadata(&path)?.modified()? > apk_modified {
            return Ok(false);
        }
        if *visited > MAX_SCAN_FILES {
            return Ok(false);
        }
    }
    Ok(true)
}

fn is_build_input(path: &Path) -> bool {
    if path
        .components()
        .any(|component| component.as_os_str() == "src")
    {
        return true;
    }
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if matches!(
        file_name,
        "gradle.properties" | "settings.gradle" | "settings.gradle.kts" | "libs.versions.toml"
    ) {
        return true;
    }
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some(
            "kt" | "kts"
                | "java"
                | "xml"
                | "gradle"
                | "properties"
                | "toml"
                | "json"
                | "png"
                | "webp"
                | "jpg"
                | "jpeg"
                | "svg"
                | "ttf"
                | "otf"
                | "aar"
                | "jar"
                | "pro"
        )
    )
}

fn collect_metadata(
    dir: &Path,
    depth: usize,
    visited: &mut usize,
    output: &mut Vec<PathBuf>,
) -> Result<()> {
    if depth > 10 || *visited > MAX_SCAN_FILES {
        return Ok(());
    }
    for entry in
        fs::read_dir(dir).with_context(|| format!("读取构建目录失败: {}", dir.display()))?
    {
        let entry = entry?;
        *visited += 1;
        let path = entry.path();
        if path.is_dir() {
            if !matches!(
                entry.file_name().to_str(),
                Some(".git" | ".gradle" | "node_modules")
            ) {
                collect_metadata(&path, depth + 1, visited, output)?;
            }
        } else if entry.file_name() == "output-metadata.json"
            && path
                .to_string_lossy()
                .replace('\\', "/")
                .contains("/build/outputs/apk/")
        {
            output.push(path);
        }
        if *visited > MAX_SCAN_FILES {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn selects_only_exact_application_id() {
        let root = test_root();
        write_output(&root, "app", "com.example.app", "app-debug.apk");
        write_output(&root, "sample", "com.example.sample", "sample-debug.apk");
        let selected = select_fresh_debug_apk(
            &root,
            "com.example.app",
            SystemTime::now() - Duration::from_secs(5),
        )
        .unwrap();
        assert_eq!(selected.file_name().unwrap(), "app-debug.apk");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_ambiguous_outputs_for_same_application() {
        let root = test_root();
        let dir = root.join("app/build/outputs/apk/debug");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("one-debug.apk"), b"one").unwrap();
        fs::write(dir.join("two-debug.apk"), b"two").unwrap();
        fs::write(
            dir.join("output-metadata.json"),
            r#"{"applicationId":"com.example.app","variantName":"debug","elements":[{"outputFile":"one-debug.apk"},{"outputFile":"two-debug.apk"}]}"#,
        )
        .unwrap();
        assert!(select_fresh_debug_apk(
            &root,
            "com.example.app",
            SystemTime::now() - Duration::from_secs(5)
        )
        .is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reuses_only_apk_not_older_than_source_inputs() {
        let root = test_root();
        fs::create_dir_all(root.join("app/src/main")).unwrap();
        fs::write(root.join("app/src/main/Main.kt"), "class Main").unwrap();
        std::thread::sleep(Duration::from_millis(30));
        write_output(&root, "app", "com.example.app", "app-debug.apk");
        assert!(select_reusable_debug_apk(&root, "com.example.app")
            .unwrap()
            .is_some());
        std::thread::sleep(Duration::from_millis(30));
        fs::write(root.join("app/src/main/Main.kt"), "class Main2").unwrap();
        assert!(select_reusable_debug_apk(&root, "com.example.app")
            .unwrap()
            .is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn accepts_timestamp_preserving_cache_output_only_after_successful_build() {
        let root = test_root();
        write_output(&root, "app", "com.example.app", "app-debug.apk");
        std::thread::sleep(Duration::from_millis(30));
        fs::write(root.join("build.gradle"), "plugins {}").unwrap();

        assert!(select_reusable_debug_apk(&root, "com.example.app")
            .unwrap()
            .is_none());
        let selected = select_debug_apk_after_successful_build(&root, "com.example.app").unwrap();
        assert_eq!(selected.file_name().unwrap(), "app-debug.apk");
        fs::remove_dir_all(root).unwrap();
    }

    fn test_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "elon-apk-selection-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_output(root: &Path, module: &str, application_id: &str, apk: &str) {
        let dir = root.join(module).join("build/outputs/apk/debug");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(apk), b"apk").unwrap();
        fs::write(
            dir.join("output-metadata.json"),
            format!(
                r#"{{"applicationId":"{application_id}","variantName":"debug","elements":[{{"outputFile":"{apk}"}}]}}"#
            ),
        )
        .unwrap();
    }
}
