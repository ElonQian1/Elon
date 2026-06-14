use super::model::RustImport;

pub(crate) fn collect_file_imports(path: &str, content: &str) -> Vec<RustImport> {
    content
        .lines()
        .enumerate()
        .flat_map(|(idx, line)| parse_use_line(path, idx + 1, line))
        .collect()
}

fn parse_use_line(path: &str, line_number: usize, line: &str) -> Vec<RustImport> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") || trimmed.starts_with("#[") {
        return Vec::new();
    }
    let (public, rest) = if let Some(rest) = strip_pub_use(trimmed) {
        (true, rest)
    } else if let Some(rest) = trimmed.strip_prefix("use ") {
        (false, rest)
    } else {
        return Vec::new();
    };
    let without_comment = rest.split("//").next().unwrap_or(rest);
    let raw_path = without_comment
        .trim()
        .trim_end_matches(';')
        .trim()
        .to_string();
    if raw_path.is_empty() {
        return Vec::new();
    }

    expand_use_tree(&raw_path)
        .into_iter()
        .map(|import_path| rust_import(path, line_number, import_path, public, trimmed))
        .collect()
}

fn strip_pub_use(line: &str) -> Option<&str> {
    if let Some(rest) = line.strip_prefix("pub use ") {
        return Some(rest);
    }
    let rest = line.strip_prefix("pub(")?;
    let (_, after_visibility) = rest.split_once(')')?;
    after_visibility.trim_start().strip_prefix("use ")
}

fn rust_import(
    path: &str,
    line_number: usize,
    raw_path: String,
    public: bool,
    raw_line: &str,
) -> RustImport {
    let (imported_path, alias) = split_alias(&raw_path);
    let glob = imported_path.ends_with("::*");
    RustImport {
        path: path.to_string(),
        line: line_number,
        imported_path,
        alias,
        public,
        glob,
        raw: raw_line.to_string(),
    }
}

fn split_alias(raw_path: &str) -> (String, Option<String>) {
    if let Some((path, alias)) = raw_path.rsplit_once(" as ") {
        return (path.trim().to_string(), Some(alias.trim().to_string()));
    }
    (raw_path.to_string(), None)
}

pub(crate) fn import_leaf(imported_path: &str, alias: Option<&str>) -> Option<String> {
    if let Some(alias) = alias.filter(|alias| !alias.is_empty()) {
        return Some(alias.to_string());
    }
    imported_path
        .trim_end_matches("::*")
        .rsplit("::")
        .next()
        .filter(|leaf| !leaf.is_empty() && *leaf != "self")
        .map(ToString::to_string)
}

fn expand_use_tree(raw_path: &str) -> Vec<String> {
    let raw_path = raw_path.trim();
    let Some((open, close)) = find_top_level_braces(raw_path) else {
        return (!raw_path.is_empty())
            .then(|| raw_path.to_string())
            .into_iter()
            .collect();
    };

    let prefix = raw_path[..open].trim().trim_end_matches("::");
    let suffix = raw_path[close + 1..].trim();
    split_top_level_commas(&raw_path[open + 1..close])
        .into_iter()
        .flat_map(|item| {
            let item = item.trim();
            if item.is_empty() {
                return Vec::new();
            }
            let joined = join_use_tree(prefix, item);
            expand_use_tree(&format!("{joined}{suffix}"))
        })
        .collect()
}

fn find_top_level_braces(value: &str) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let mut open = None;
    for (idx, ch) in value.char_indices() {
        match ch {
            '{' => {
                if depth == 0 {
                    open = Some(idx);
                }
                depth += 1;
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return open.map(|open| (open, idx));
                }
            }
            _ => {}
        }
    }
    None
}

fn split_top_level_commas(value: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (idx, ch) in value.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(&value[start..idx]);
                start = idx + 1;
            }
            _ => {}
        }
    }
    parts.push(&value[start..]);
    parts
}

fn join_use_tree(prefix: &str, item: &str) -> String {
    let item = item.trim();
    if item == "self" {
        return prefix.to_string();
    }
    if prefix.is_empty() {
        item.to_string()
    } else {
        format!("{prefix}::{item}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_use_and_public_reexport_lines() {
        let imports = collect_file_imports(
            "src/lib.rs",
            r#"
use crate::domain::User as DomainUser;
pub(crate) use crate::{service::*, domain::{Account, Profile as UserProfile}};
"#,
        );

        assert_eq!(imports.len(), 4);
        assert_eq!(imports[0].imported_path, "crate::domain::User");
        assert_eq!(imports[0].alias.as_deref(), Some("DomainUser"));
        assert!(!imports[0].public);
        assert!(imports[1].public);
        assert!(imports[1].glob);
        assert_eq!(imports[2].imported_path, "crate::domain::Account");
        assert_eq!(imports[3].alias.as_deref(), Some("UserProfile"));
        assert_eq!(
            import_leaf(&imports[0].imported_path, imports[0].alias.as_deref()).as_deref(),
            Some("DomainUser")
        );
    }
}
