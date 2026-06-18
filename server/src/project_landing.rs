use serde_json::{json, Map, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

const MANIFEST_PATHS: [&str; 2] = [".elon/project-landing.json", ".elon/landing.json"];
const MAX_MANIFEST_BYTES: u64 = 256 * 1024;
const MAX_SHORT_TEXT: usize = 160;
const MAX_LONG_TEXT: usize = 1200;
const MAX_URL: usize = 2048;
const MAX_ITEMS: usize = 12;

pub(crate) fn load_workspace_landing(workspace: &Path) -> Option<Value> {
    for relative_path in MANIFEST_PATHS {
        let manifest_path = workspace.join(relative_path);
        if manifest_path.is_file() {
            return Some(load_manifest_file(&manifest_path, relative_path));
        }
    }
    None
}

#[allow(dead_code)]
pub(crate) fn normalize_landing_snapshot(value: &Value) -> Option<Value> {
    let mut landing = normalize_manifest(value.clone())?;
    if !has_display_content(&Value::Object(landing.clone())) {
        return None;
    }
    landing.insert(
        "source".to_string(),
        json!({
            "mode": "node_agent_snapshot",
            "status": "available",
        }),
    );
    Some(Value::Object(landing))
}

#[allow(dead_code)]
pub(crate) fn has_display_content(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    object.iter().any(|(key, value)| {
        key != "source"
            && match value {
                Value::Null => false,
                Value::String(value) => !value.trim().is_empty(),
                Value::Array(value) => !value.is_empty(),
                Value::Object(value) => !value.is_empty(),
                Value::Bool(_) | Value::Number(_) => true,
            }
    })
}

fn load_manifest_file(path: &Path, relative_path: &str) -> Value {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => return source_only(relative_path, "invalid", Some(error.to_string())),
    };
    if metadata.len() > MAX_MANIFEST_BYTES {
        return source_only(
            relative_path,
            "invalid",
            Some(format!("manifest 超过 {} KB", MAX_MANIFEST_BYTES / 1024)),
        );
    }

    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return source_only(relative_path, "invalid", Some(error.to_string())),
    };
    let parsed = match serde_json::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(error) => return source_only(relative_path, "invalid", Some(error.to_string())),
    };
    let mut landing = match normalize_manifest(parsed) {
        Some(landing) => landing,
        None => {
            return source_only(
                relative_path,
                "invalid",
                Some("manifest 根节点必须是对象".to_string()),
            )
        }
    };
    landing.insert(
        "source".to_string(),
        source_value(relative_path, "available", None),
    );
    Value::Object(landing)
}

fn normalize_manifest(value: Value) -> Option<Map<String, Value>> {
    let object = value.as_object()?;
    let mut output = Map::new();

    insert_string(
        &mut output,
        "schema_version",
        first_string(object, &["schema_version", "schemaVersion"], MAX_SHORT_TEXT),
    );
    insert_string(
        &mut output,
        "title",
        first_string(
            object,
            &["title", "name", "display_name", "displayName"],
            MAX_SHORT_TEXT,
        ),
    );
    insert_string(
        &mut output,
        "tagline",
        first_string(object, &["tagline", "slogan", "subtitle"], MAX_SHORT_TEXT),
    );
    insert_string(
        &mut output,
        "summary",
        first_string(object, &["summary", "overview"], MAX_LONG_TEXT),
    );
    insert_string(
        &mut output,
        "description",
        first_string(
            object,
            &["description", "intro", "introduction"],
            MAX_LONG_TEXT,
        ),
    );
    insert_string(
        &mut output,
        "web_url",
        first_url(
            object,
            &[
                "web_url",
                "webUrl",
                "website_url",
                "websiteUrl",
                "homepage_url",
            ],
        ),
    );
    insert_string(
        &mut output,
        "custom_landing_url",
        first_url(
            object,
            &[
                "custom_landing_url",
                "customLandingUrl",
                "promote_url",
                "official_url",
            ],
        ),
    );
    insert_string(
        &mut output,
        "landing_manifest_url",
        first_url(
            object,
            &["landing_manifest_url", "landingManifestUrl", "manifest_url"],
        ),
    );

    insert_text_array(
        &mut output,
        "highlights",
        first_array(object, &["highlights", "features", "selling_points"]),
    );
    insert_text_array(
        &mut output,
        "target_users",
        first_array(object, &["target_users", "targetUsers", "audience"]),
    );
    insert_text_array(
        &mut output,
        "recent_updates",
        first_array(
            object,
            &[
                "recent_updates",
                "recentUpdates",
                "release_notes",
                "releaseNotes",
            ],
        ),
    );
    insert_text_array(
        &mut output,
        "system_requirements",
        first_array(
            object,
            &["system_requirements", "systemRequirements", "requirements"],
        ),
    );
    insert_text_array(
        &mut output,
        "privacy_notes",
        first_array(
            object,
            &["privacy_notes", "privacyNotes", "privacy", "permissions"],
        ),
    );

    let downloads = normalize_downloads(object.get("downloads"));
    if !downloads.is_empty() {
        output.insert("downloads".to_string(), Value::Array(downloads));
    }

    let media = normalize_named_urls(first_value(object, &["media", "screenshots", "videos"]));
    if !media.is_empty() {
        output.insert("media".to_string(), Value::Array(media));
    }

    let resources = normalize_named_urls(first_value(object, &["resources", "links"]));
    if !resources.is_empty() {
        output.insert("resources".to_string(), Value::Array(resources));
    }

    if let Some(sections) = normalize_sections(object.get("sections")) {
        output.insert("sections".to_string(), sections);
    }

    Some(output)
}

fn normalize_sections(value: Option<&Value>) -> Option<Value> {
    let object = value?.as_object()?;
    let mut output = Map::new();
    for (key, value) in object.iter().take(MAX_ITEMS) {
        let clean_key = clean_text_value(&Value::String(key.clone()), MAX_SHORT_TEXT)?;
        if let Some(items) = text_array_from_value(value) {
            if !items.is_empty() {
                output.insert(clean_key, Value::Array(items));
            }
            continue;
        }
        if let Some(text) = clean_text_value(value, MAX_LONG_TEXT) {
            output.insert(clean_key, Value::String(text));
        }
    }
    (!output.is_empty()).then_some(Value::Object(output))
}

fn normalize_downloads(value: Option<&Value>) -> Vec<Value> {
    let Some(value) = value else {
        return Vec::new();
    };
    let mut downloads = Vec::new();
    match value {
        Value::Array(items) => {
            for item in items.iter().take(MAX_ITEMS) {
                if let Some(download) = normalize_download(item, None) {
                    downloads.push(download);
                }
            }
        }
        Value::Object(items) => {
            for (platform, item) in items.iter().take(MAX_ITEMS) {
                if let Some(download) = normalize_download(item, Some(platform)) {
                    downloads.push(download);
                }
            }
        }
        _ => {}
    }
    downloads
}

fn normalize_download(value: &Value, platform_hint: Option<&str>) -> Option<Value> {
    let mut object = Map::new();
    match value {
        Value::Object(source) => {
            let platform = normalize_platform(
                first_string(source, &["platform", "os", "kind", "type"], MAX_SHORT_TEXT)
                    .as_deref()
                    .or(platform_hint),
            )?;
            object.insert("platform".to_string(), Value::String(platform));
            insert_string(
                &mut object,
                "label",
                first_string(source, &["label", "name", "title"], MAX_SHORT_TEXT),
            );
            insert_string(
                &mut object,
                "short",
                first_string(source, &["short", "badge"], MAX_SHORT_TEXT),
            );
            insert_string(
                &mut object,
                "url",
                first_url(source, &["url", "download_url", "downloadUrl", "href"]),
            );
            insert_string(
                &mut object,
                "manifest_url",
                first_url(source, &["manifest_url", "manifestUrl"]),
            );
            insert_string(
                &mut object,
                "version",
                first_string(
                    source,
                    &["version", "version_name", "versionName", "build"],
                    MAX_SHORT_TEXT,
                ),
            );
            insert_string(
                &mut object,
                "size_label",
                first_string(source, &["size_label", "sizeLabel", "size"], MAX_SHORT_TEXT),
            );
            insert_string(
                &mut object,
                "size_bytes",
                first_string(
                    source,
                    &["size_bytes", "sizeBytes", "file_size", "fileSize"],
                    MAX_SHORT_TEXT,
                ),
            );
            insert_string(
                &mut object,
                "note",
                first_string(
                    source,
                    &[
                        "note",
                        "description",
                        "health_error",
                        "healthError",
                        "changelog",
                        "release_notes",
                        "releaseNotes",
                    ],
                    MAX_LONG_TEXT,
                ),
            );
            insert_bool(&mut object, "external", source.get("external"));
            let status = normalize_status(
                first_string(
                    source,
                    &["status", "availability", "health_status", "healthStatus"],
                    MAX_SHORT_TEXT,
                )
                .as_deref(),
                object.get("url").and_then(Value::as_str),
                object.get("manifest_url").and_then(Value::as_str),
            );
            object.insert("status".to_string(), Value::String(status));
        }
        Value::String(url) => {
            let platform = normalize_platform(platform_hint)?;
            object.insert("platform".to_string(), Value::String(platform));
            if let Some(url) = clean_url(url) {
                object.insert("url".to_string(), Value::String(url));
                object.insert("status".to_string(), Value::String("available".to_string()));
            } else {
                object.insert("status".to_string(), Value::String("planned".to_string()));
            }
        }
        _ => return None,
    }
    Some(Value::Object(object))
}

fn normalize_named_urls(value: Option<&Value>) -> Vec<Value> {
    let Some(value) = value else {
        return Vec::new();
    };
    let values: Vec<&Value> = match value {
        Value::Array(items) => items.iter().take(MAX_ITEMS).collect(),
        Value::Object(_) => vec![value],
        _ => Vec::new(),
    };
    values
        .into_iter()
        .filter_map(|item| {
            let object = item.as_object()?;
            let url = first_url(object, &["url", "href", "link", "src"])?;
            let mut output = Map::new();
            output.insert("url".to_string(), Value::String(url));
            insert_string(
                &mut output,
                "label",
                first_string(object, &["label", "name", "title", "alt"], MAX_SHORT_TEXT),
            );
            insert_string(
                &mut output,
                "kind",
                first_string(object, &["kind", "type"], MAX_SHORT_TEXT),
            );
            insert_string(
                &mut output,
                "note",
                first_string(object, &["note", "description"], MAX_LONG_TEXT),
            );
            Some(Value::Object(output))
        })
        .collect()
}

fn insert_text_array(output: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    if let Some(items) = value.and_then(text_array_from_value) {
        if !items.is_empty() {
            output.insert(key.to_string(), Value::Array(items));
        }
    }
}

fn text_array_from_value(value: &Value) -> Option<Vec<Value>> {
    match value {
        Value::Array(items) => Some(
            items
                .iter()
                .take(MAX_ITEMS)
                .filter_map(|item| {
                    if let Some(text) = clean_text_value(item, MAX_LONG_TEXT) {
                        return Some(Value::String(text));
                    }
                    let object = item.as_object()?;
                    first_string(object, &["title", "text", "label", "name"], MAX_LONG_TEXT)
                        .map(Value::String)
                })
                .collect(),
        ),
        Value::String(_) => {
            clean_text_value(value, MAX_LONG_TEXT).map(|text| vec![Value::String(text)])
        }
        _ => None,
    }
}

fn insert_string(output: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        output.insert(key.to_string(), Value::String(value));
    }
}

fn insert_bool(output: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    if let Some(value) = value.and_then(Value::as_bool) {
        output.insert(key.to_string(), Value::Bool(value));
    }
}

fn first_value<'a>(object: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| object.get(*key))
}

fn first_array<'a>(object: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    first_value(object, keys).filter(|value| value.is_array() || value.is_string())
}

fn first_string(object: &Map<String, Value>, keys: &[&str], max_chars: usize) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key))
        .and_then(|value| clean_text_value(value, max_chars))
}

fn first_url(object: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key))
        .and_then(|value| clean_text_value(value, MAX_URL))
        .and_then(|value| clean_url(&value))
}

fn clean_text_value(value: &Value, max_chars: usize) -> Option<String> {
    let raw = match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        _ => return None,
    };
    let cleaned = raw
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .collect::<String>();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trim_chars(trimmed, max_chars))
}

fn trim_chars(value: &str, max_chars: usize) -> String {
    let mut iter = value.chars();
    let trimmed: String = iter.by_ref().take(max_chars).collect();
    if iter.next().is_some() {
        format!("{trimmed}...")
    } else {
        trimmed
    }
}

fn clean_url(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let lower = value.to_ascii_lowercase();
    let safe = lower.starts_with("https://")
        || lower.starts_with("http://")
        || (value.starts_with('/') && !value.starts_with("//"));
    safe.then(|| trim_chars(value, MAX_URL))
}

fn normalize_platform(value: Option<&str>) -> Option<String> {
    let raw = value?.trim().to_ascii_lowercase();
    let compact = raw
        .chars()
        .filter(|ch| !matches!(ch, '_' | '-' | ' '))
        .collect::<String>();
    let platform = match compact.as_str() {
        "android" | "apk" | "androidapk" => "android",
        "windows" | "win" | "window" | "windowsclient" => "windows",
        "web" | "browser" | "h5" | "website" => "web",
        "ios" | "iphone" | "ipad" => "ios",
        "macos" | "mac" | "osx" | "darwin" => "macos",
        "linux" => "linux",
        _ => return None,
    };
    Some(platform.to_string())
}

fn normalize_status(value: Option<&str>, url: Option<&str>, manifest_url: Option<&str>) -> String {
    let explicit = value
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match explicit.as_str() {
        "available"
        | "external"
        | "unavailable"
        | "coming_soon"
        | "needs_configuration"
        | "third_party"
        | "planned"
        | "pending" => explicit,
        _ if url.is_some() => "available".to_string(),
        _ if manifest_url.is_some() => "pending".to_string(),
        _ => "planned".to_string(),
    }
}

fn source_only(relative_path: &str, status: &str, error: Option<String>) -> Value {
    json!({
        "source": source_value(relative_path, status, error),
    })
}

fn source_value(relative_path: &str, status: &str, error: Option<String>) -> Value {
    let mut source = Map::new();
    source.insert(
        "mode".to_string(),
        Value::String("workspace_manifest".to_string()),
    );
    source.insert("status".to_string(), Value::String(status.to_string()));
    source.insert("file".to_string(), Value::String(relative_path.to_string()));
    if let Some(error) =
        error.and_then(|value| clean_text_value(&Value::String(value), MAX_LONG_TEXT))
    {
        source.insert("health_error".to_string(), Value::String(error));
    }
    Value::Object(source)
}

#[allow(dead_code)]
fn manifest_path(workspace: &Path, relative_path: &str) -> PathBuf {
    workspace.join(relative_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn load_workspace_landing_normalizes_manifest() {
        let workspace = temp_workspace("normalizes");
        let manifest_dir = workspace.join(".elon");
        fs::create_dir_all(&manifest_dir).unwrap();
        fs::write(
            manifest_dir.join("project-landing.json"),
            r#"{
              "title": "Demo",
              "tagline": "Fast client downloads",
              "downloads": {
                "apk": {
                  "url": "https://example.com/app.apk",
                  "version_name": "1.2.3",
                  "size": "45 MB"
                },
                "ios": {
                  "url": "javascript:alert(1)",
                  "status": "unavailable",
                  "health_error": "guide 404"
                }
              },
              "resources": [
                { "label": "官网", "url": "https://example.com" },
                { "label": "bad", "url": "javascript:alert(1)" }
              ],
              "target_users": ["新用户", { "title": "团队成员" }]
            }"#,
        )
        .unwrap();

        let landing = load_workspace_landing(&workspace).unwrap();
        let object = landing.as_object().unwrap();
        assert_eq!(object["title"], "Demo");
        assert_eq!(object["source"]["status"], "available");
        assert_eq!(object["downloads"][0]["platform"], "android");
        assert_eq!(object["downloads"][0]["status"], "available");
        assert_eq!(object["downloads"][1]["platform"], "ios");
        assert!(object["downloads"][1].get("url").is_none());
        assert_eq!(object["resources"].as_array().unwrap().len(), 1);
        assert_eq!(object["target_users"].as_array().unwrap().len(), 2);

        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn invalid_manifest_returns_source_error_without_absolute_path() {
        let workspace = temp_workspace("invalid");
        let manifest_dir = workspace.join(".elon");
        fs::create_dir_all(&manifest_dir).unwrap();
        fs::write(manifest_dir.join("project-landing.json"), "{").unwrap();

        let landing = load_workspace_landing(&workspace).unwrap();
        assert_eq!(landing["source"]["status"], "invalid");
        assert_eq!(landing["source"]["file"], ".elon/project-landing.json");
        assert!(landing["source"]["health_error"]
            .as_str()
            .unwrap()
            .contains("EOF"));
        assert!(!landing["source"]["health_error"]
            .as_str()
            .unwrap()
            .contains(workspace.to_string_lossy().as_ref()));

        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn normalize_landing_snapshot_rewrites_source_and_filters_urls() {
        let snapshot = normalize_landing_snapshot(&json!({
            "source": { "mode": "workspace_manifest", "status": "available", "file": ".elon/project-landing.json" },
            "title": "Demo",
            "downloads": {
                "windows": {
                    "downloadUrl": "https://example.com/app.exe",
                    "fileSize": 7141343,
                    "changelog": "fix tun"
                },
                "ios": {
                    "status": "needs_configuration",
                    "url": "http://example.com/ios.html"
                }
            }
        }))
        .unwrap();

        assert_eq!(snapshot["source"]["mode"], "node_agent_snapshot");
        assert_eq!(snapshot["title"], "Demo");
        let downloads = snapshot["downloads"].as_array().unwrap();
        let windows = downloads
            .iter()
            .find(|download| download["platform"] == "windows")
            .unwrap();
        let ios = downloads
            .iter()
            .find(|download| download["platform"] == "ios")
            .unwrap();
        assert_eq!(
            windows["url"],
            "https://example.com/app.exe"
        );
        assert_eq!(windows["size_bytes"], "7141343");
        assert_eq!(windows["note"], "fix tun");
        assert_eq!(ios["status"], "needs_configuration");
        assert!(has_display_content(&snapshot));
    }

    fn temp_workspace(label: &str) -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("elon-project-landing-{label}-{id}"))
    }
}
