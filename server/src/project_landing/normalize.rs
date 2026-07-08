use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

use super::{MANIFEST_PATHS, MAX_ITEMS, MAX_LONG_TEXT, MAX_SHORT_TEXT, MAX_URL, MAX_VARIANTS};
use super::normalize_helpers::*;

pub(super) fn normalize_manifest(value: Value) -> Option<Map<String, Value>> {
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
    insert_string(
        &mut output,
        "release_manifest_url",
        first_url(object, &["release_manifest_url", "releaseManifestUrl"]),
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

pub(super) fn normalize_sections(value: Option<&Value>) -> Option<Value> {
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

pub(super) fn normalize_downloads(value: Option<&Value>) -> Vec<Value> {
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

pub(super) fn normalize_download(value: &Value, platform_hint: Option<&str>) -> Option<Value> {
    let mut object = Map::new();
    match value {
        Value::Object(source) => {
            let platform = download_platform(source, platform_hint)?;
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
                "kind",
                first_string(
                    source,
                    &["kind", "client_kind", "clientKind"],
                    MAX_SHORT_TEXT,
                ),
            );
            insert_string(
                &mut object,
                "url",
                first_url(
                    source,
                    &[
                        "url",
                        "download_url",
                        "downloadUrl",
                        "fallback_url",
                        "fallbackUrl",
                        "href",
                    ],
                ),
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
            let variants =
                normalize_download_variants(first_value(source, &["options", "variants"]));
            if !variants.is_empty() {
                object.insert("variants".to_string(), Value::Array(variants.clone()));
            }
            let base_status = normalize_status(
                first_string(
                    source,
                    &["status", "availability", "health_status", "healthStatus"],
                    MAX_SHORT_TEXT,
                )
                .as_deref(),
                object.get("url").and_then(Value::as_str),
                object.get("manifest_url").and_then(Value::as_str),
            );
            let status = if variants.is_empty() {
                base_status
            } else {
                aggregate_variant_status(&variants, &base_status)
            };
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

pub(super) fn download_platform(
    source: &Map<String, Value>,
    platform_hint: Option<&str>,
) -> Option<String> {
    first_string(source, &["platform", "os", "type"], MAX_SHORT_TEXT)
        .as_deref()
        .and_then(|value| normalize_platform(Some(value)))
        .or_else(|| normalize_platform(platform_hint))
        .or_else(|| {
            first_string(source, &["kind"], MAX_SHORT_TEXT)
                .as_deref()
                .and_then(|value| normalize_platform(Some(value)))
        })
}

pub(super) fn normalize_download_variants(value: Option<&Value>) -> Vec<Value> {
    let Some(value) = value else {
        return Vec::new();
    };
    let mut variants = Vec::new();
    match value {
        Value::Array(items) => {
            for item in items.iter().take(MAX_VARIANTS) {
                if let Some(variant) = normalize_download_variant(item, None) {
                    variants.push(variant);
                }
            }
        }
        Value::Object(items) => {
            for (id, item) in items.iter().take(MAX_VARIANTS) {
                if let Some(variant) = normalize_download_variant(item, Some(id)) {
                    variants.push(variant);
                }
            }
        }
        _ => {}
    }
    variants
}

pub(super) fn normalize_download_variant(value: &Value, id_hint: Option<&str>) -> Option<Value> {
    let mut output = Map::new();
    match value {
        Value::Object(source) => {
            insert_string(
                &mut output,
                "id",
                first_string(source, &["id", "key"], MAX_SHORT_TEXT).or_else(|| {
                    id_hint.and_then(|id| {
                        clean_text_value(&Value::String(id.to_string()), MAX_SHORT_TEXT)
                    })
                }),
            );
            insert_string(
                &mut output,
                "label",
                first_string(source, &["label", "name"], MAX_SHORT_TEXT),
            );
            insert_string(
                &mut output,
                "arch",
                first_string(source, &["arch", "architecture", "cpu"], MAX_SHORT_TEXT),
            );
            insert_string(
                &mut output,
                "title",
                first_string(source, &["title"], MAX_SHORT_TEXT),
            );
            insert_string(
                &mut output,
                "url",
                first_url(
                    source,
                    &[
                        "url",
                        "download_url",
                        "downloadUrl",
                        "fallback_url",
                        "fallbackUrl",
                        "href",
                    ],
                ),
            );
            insert_string(
                &mut output,
                "version",
                first_string(
                    source,
                    &["version", "version_name", "versionName", "build"],
                    MAX_SHORT_TEXT,
                ),
            );
            insert_string(
                &mut output,
                "size_label",
                first_string(source, &["size_label", "sizeLabel", "size"], MAX_SHORT_TEXT),
            );
            insert_string(
                &mut output,
                "size_bytes",
                first_string(
                    source,
                    &["size_bytes", "sizeBytes", "file_size", "fileSize"],
                    MAX_SHORT_TEXT,
                ),
            );
            insert_string(
                &mut output,
                "health_error",
                first_string(
                    source,
                    &["health_error", "healthError", "error"],
                    MAX_LONG_TEXT,
                ),
            );
            insert_string(
                &mut output,
                "note",
                first_string(
                    source,
                    &[
                        "note",
                        "description",
                        "changelog",
                        "release_notes",
                        "releaseNotes",
                    ],
                    MAX_LONG_TEXT,
                ),
            );
            let status = normalize_status(
                first_string(
                    source,
                    &["status", "availability", "health_status", "healthStatus"],
                    MAX_SHORT_TEXT,
                )
                .as_deref(),
                output.get("url").and_then(Value::as_str),
                None,
            );
            output.insert("status".to_string(), Value::String(status));
        }
        Value::String(url) => {
            insert_string(
                &mut output,
                "id",
                id_hint.and_then(|id| {
                    clean_text_value(&Value::String(id.to_string()), MAX_SHORT_TEXT)
                }),
            );
            if let Some(url) = clean_url(url) {
                output.insert("url".to_string(), Value::String(url));
                output.insert("status".to_string(), Value::String("available".to_string()));
            } else {
                output.insert("status".to_string(), Value::String("planned".to_string()));
            }
        }
        _ => return None,
    }
    Some(Value::Object(output))
}

