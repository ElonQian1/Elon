use super::{pwa_resolver::resolve_pwa_style_binding, types::ResolvePwaStyleBindingRequest};
use std::{fs, path::PathBuf};
use uuid::Uuid;

fn workspace() -> PathBuf {
    let root = std::env::temp_dir().join(format!("elon-pwa-resolver-{}", Uuid::new_v4()));
    fs::create_dir_all(root.join("src")).expect("create workspace");
    root
}

#[test]
fn resolves_unique_css_rule_inside_html() {
    let root = workspace();
    let source = "<html><style>\n.toolbar h1 { font-size: 16px; color: white; }\n</style></html>";
    fs::write(root.join("src/index.html"), source).expect("write html");
    let response = resolve_pwa_style_binding(&ResolvePwaStyleBindingRequest {
        project_root: root.to_string_lossy().into_owned(),
        selectors: vec![".toolbar h1".into()],
    })
    .expect("resolve binding");
    let binding = response.binding.expect("unique binding");
    assert_eq!(binding.source_file, "src/index.html");
    assert_eq!(binding.target, ".toolbar h1");
    assert_eq!(
        &source[binding.range.start..binding.range.end],
        ".toolbar h1 { font-size: 16px; color: white; }"
    );
    assert_eq!(
        binding.property_map.get("fontSize").map(String::as_str),
        Some("font-size")
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn refuses_ambiguous_rules() {
    let root = workspace();
    fs::write(root.join("src/a.css"), ".title { font-size: 16px; }").expect("write a");
    fs::write(root.join("src/b.css"), ".title { font-size: 18px; }").expect("write b");
    let response = resolve_pwa_style_binding(&ResolvePwaStyleBindingRequest {
        project_root: root.to_string_lossy().into_owned(),
        selectors: vec![".title".into()],
    })
    .expect("resolve candidates");
    assert!(response.binding.is_none());
    assert_eq!(response.candidate_count, 2);
    fs::remove_dir_all(root).ok();
}
