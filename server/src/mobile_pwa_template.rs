use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{OnceLock, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

const TEMPLATE_RELATIVE_PATH: &str = "mobile-pwa/web_page.html";

#[derive(Clone, Debug, Eq, PartialEq)]
struct TemplateSignature {
    length: u64,
    modified_nanos: u128,
}

#[derive(Clone)]
struct CachedTemplate {
    signature: Option<TemplateSignature>,
    rendered: String,
    runtime_source: bool,
}

pub(crate) struct MobilePwaPage {
    pub(crate) html: String,
    pub(crate) source: &'static str,
}

static CACHE: OnceLock<RwLock<HashMap<PathBuf, CachedTemplate>>> = OnceLock::new();

pub(crate) fn load_mobile_pwa_page<F>(
    data_dir: &Path,
    embedded_template: &str,
    render: F,
) -> MobilePwaPage
where
    F: FnOnce(&str) -> String,
{
    let path = data_dir.join(TEMPLATE_RELATIVE_PATH);
    let signature = template_signature(&path);
    let cache = CACHE.get_or_init(|| RwLock::new(HashMap::new()));

    if let Ok(entries) = cache.read() {
        if let Some(entry) = entries.get(&path) {
            if entry.signature == signature {
                return page_from_cache(entry);
            }
        }
    }

    let runtime_template = signature
        .as_ref()
        .and_then(|_| fs::read_to_string(&path).ok());
    let runtime_source = runtime_template.is_some();
    let template = runtime_template.as_deref().unwrap_or(embedded_template);
    let entry = CachedTemplate {
        signature,
        rendered: render(template),
        runtime_source,
    };

    if let Ok(mut entries) = cache.write() {
        entries.insert(path, entry.clone());
    }
    page_from_cache(&entry)
}

fn page_from_cache(entry: &CachedTemplate) -> MobilePwaPage {
    MobilePwaPage {
        html: entry.rendered.clone(),
        source: if entry.runtime_source {
            "runtime"
        } else {
            "embedded"
        },
    }
}

fn template_signature(path: &Path) -> Option<TemplateSignature> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    Some(TemplateSignature {
        length: metadata.len(),
        modified_nanos: modified_nanos(metadata.modified().ok()),
    })
}

fn modified_nanos(modified: Option<SystemTime>) -> u128 {
    modified
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::load_mobile_pwa_page;
    use std::{fs, time::SystemTime};

    fn temp_dir(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "elon-mobile-pwa-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn falls_back_to_embedded_template_and_reloads_runtime_template() {
        let root = temp_dir("reload");
        fs::create_dir_all(root.join("mobile-pwa")).unwrap();
        let first = load_mobile_pwa_page(&root, "embedded", |value| format!("render:{value}"));
        assert_eq!(first.source, "embedded");
        assert_eq!(first.html, "render:embedded");

        fs::write(root.join("mobile-pwa/web_page.html"), "runtime-template").unwrap();
        let second = load_mobile_pwa_page(&root, "embedded", |value| format!("render:{value}"));
        assert_eq!(second.source, "runtime");
        assert_eq!(second.html, "render:runtime-template");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn caches_rendered_template_until_file_signature_changes() {
        let root = temp_dir("cache");
        fs::create_dir_all(root.join("mobile-pwa")).unwrap();
        let path = root.join("mobile-pwa/web_page.html");
        fs::write(&path, "one").unwrap();
        let first = load_mobile_pwa_page(&root, "embedded", |value| value.to_owned());
        let cached = load_mobile_pwa_page(&root, "embedded", |_| panic!("must use cache"));
        assert_eq!(first.html, cached.html);

        fs::write(&path, "a-longer-template").unwrap();
        let changed = load_mobile_pwa_page(&root, "embedded", |value| value.to_owned());
        assert_eq!(changed.html, "a-longer-template");
        fs::remove_dir_all(root).unwrap();
    }
}
