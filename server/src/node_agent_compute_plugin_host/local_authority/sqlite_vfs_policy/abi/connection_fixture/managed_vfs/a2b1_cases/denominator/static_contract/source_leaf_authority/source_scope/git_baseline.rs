use std::{
    path::Path,
    process::{Command, Output},
};

use super::super::source_scope_support::is_lower_hex;
use super::SOURCE_BASELINE_COMMIT_SHA1;

const MANIFEST_REPOSITORY_PREFIX: &str = "server/";

pub(crate) fn validate_baseline_path_blob(
    repo_relative_path: &str,
    git_blob_oid_sha1: &str,
) -> Result<(), String> {
    validate_baseline_path_blobs(std::iter::once((repo_relative_path, git_blob_oid_sha1)))
}

pub(crate) fn validate_baseline_path_blobs<'a>(
    entries: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<(), String> {
    let entries = entries.into_iter().collect::<Vec<_>>();
    if entries.is_empty() {
        return Err("baseline tree validation received no source paths".to_owned());
    }
    for (path, oid) in &entries {
        validate_source_path(path)?;
        if !is_lower_hex(oid, 40) {
            return Err(format!("baseline tree blob OID is invalid for {path}"));
        }
    }

    let repository = GitRepository::validate_current_baseline()?;
    for (path, oid) in entries {
        repository.validate_tree_blob(path, oid)?;
    }
    Ok(())
}

struct GitRepository {
    manifest_dir: &'static Path,
}

impl GitRepository {
    fn validate_current_baseline() -> Result<Self, String> {
        let repository = Self {
            manifest_dir: Path::new(env!("CARGO_MANIFEST_DIR")),
        };
        if !repository.manifest_dir.is_dir() {
            return Err("CARGO_MANIFEST_DIR is not an accessible directory".to_owned());
        }

        let prefix_output = repository.git(&["rev-parse", "--show-prefix"])?;
        let prefix = parse_single_lf_line("repository prefix", &prefix_output.stdout)?;
        if prefix != MANIFEST_REPOSITORY_PREFIX {
            return Err(format!(
                "server manifest repository prefix drifted: expected {MANIFEST_REPOSITORY_PREFIX:?}, got {prefix:?}"
            ));
        }

        let revision = format!("{SOURCE_BASELINE_COMMIT_SHA1}^{{commit}}");
        let resolved_output = repository.git(&["rev-parse", "--verify", revision.as_str()])?;
        let resolved = parse_single_lf_line("resolved source baseline", &resolved_output.stdout)?;
        let ancestor = repository.baseline_is_head_ancestor()?;
        validate_baseline_relation(SOURCE_BASELINE_COMMIT_SHA1, resolved, ancestor)?;
        Ok(repository)
    }

    fn baseline_is_head_ancestor(&self) -> Result<bool, String> {
        let output = self.git_allow_status(&[
            "merge-base",
            "--is-ancestor",
            SOURCE_BASELINE_COMMIT_SHA1,
            "HEAD",
        ])?;
        require_empty_streams("git merge-base --is-ancestor", &output)?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err("git merge-base --is-ancestor failed unexpectedly".to_owned()),
        }
    }

    fn validate_tree_blob(&self, source_path: &str, expected_oid: &str) -> Result<(), String> {
        let tree_path = format!("{MANIFEST_REPOSITORY_PREFIX}{source_path}");
        let output = self.git(&[
            "-c",
            "core.quotePath=false",
            "ls-tree",
            "-z",
            "--full-tree",
            SOURCE_BASELINE_COMMIT_SHA1,
            "--",
            tree_path.as_str(),
        ])?;
        parse_ls_tree_blob(&output.stdout, &tree_path, expected_oid)
    }

    fn git(&self, arguments: &[&str]) -> Result<Output, String> {
        let output = self.git_allow_status(arguments)?;
        if !output.status.success() {
            return Err(format!("git {} failed", command_label(arguments)));
        }
        if !output.stderr.is_empty() {
            return Err(format!(
                "git {} emitted unexpected stderr",
                command_label(arguments)
            ));
        }
        Ok(output)
    }

    fn git_allow_status(&self, arguments: &[&str]) -> Result<Output, String> {
        Command::new("git")
            .current_dir(self.manifest_dir)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .args(arguments)
            .output()
            .map_err(|error| {
                format!(
                    "failed to execute git {}: {error}",
                    command_label(arguments)
                )
            })
    }
}

fn validate_source_path(path: &str) -> Result<(), String> {
    if !path.starts_with("src/")
        || path.contains('\0')
        || path.contains('\\')
        || path.contains('\n')
        || path.contains('\r')
        || path
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(format!("invalid baseline source path: {path:?}"));
    }
    Ok(())
}

fn validate_baseline_relation(
    declared: &str,
    resolved: &str,
    is_head_ancestor: bool,
) -> Result<(), String> {
    if !is_lower_hex(declared, 40) || !is_lower_hex(resolved, 40) || resolved != declared {
        return Err("declared source baseline did not resolve to the exact commit".to_owned());
    }
    if !is_head_ancestor {
        return Err("declared source baseline is not an ancestor of current HEAD".to_owned());
    }
    Ok(())
}

fn parse_single_lf_line<'a>(label: &str, bytes: &'a [u8]) -> Result<&'a str, String> {
    let line = bytes
        .strip_suffix(b"\n")
        .ok_or_else(|| format!("{label} is not one LF-terminated line"))?;
    if line.is_empty() || line.contains(&b'\n') || line.contains(&b'\r') || line.contains(&0) {
        return Err(format!("{label} is not one canonical line"));
    }
    std::str::from_utf8(line).map_err(|_| format!("{label} is not UTF-8"))
}

fn parse_ls_tree_blob(
    output: &[u8],
    expected_path: &str,
    expected_oid: &str,
) -> Result<(), String> {
    let record = output
        .strip_suffix(&[0])
        .ok_or_else(|| format!("baseline tree entry is absent for {expected_path}"))?;
    if record.is_empty() || record.contains(&0) {
        return Err(format!(
            "baseline tree returned zero or multiple entries for {expected_path}"
        ));
    }
    let record = std::str::from_utf8(record)
        .map_err(|_| format!("baseline tree entry is not UTF-8 for {expected_path}"))?;
    let (header, actual_path) = record
        .split_once('\t')
        .ok_or_else(|| format!("baseline tree entry has no path for {expected_path}"))?;
    let mut fields = header.split(' ');
    let mode = fields.next().unwrap_or_default();
    let kind = fields.next().unwrap_or_default();
    let actual_oid = fields.next().unwrap_or_default();
    if fields.next().is_some()
        || mode.len() != 6
        || !mode.bytes().all(|byte| (b'0'..=b'7').contains(&byte))
        || kind != "blob"
        || !is_lower_hex(actual_oid, 40)
        || actual_path != expected_path
        || actual_oid != expected_oid
    {
        return Err(format!(
            "baseline tree blob binding drifted for {expected_path}"
        ));
    }
    Ok(())
}

fn require_empty_streams(label: &str, output: &Output) -> Result<(), String> {
    if !output.stdout.is_empty() || !output.stderr.is_empty() {
        return Err(format!("{label} emitted unexpected output"));
    }
    Ok(())
}

fn command_label<'a>(arguments: &'a [&'a str]) -> &'a str {
    arguments
        .iter()
        .copied()
        .find(|argument| !argument.starts_with('-') && *argument != "core.quotePath=false")
        .unwrap_or("command")
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASELINE: &str = "47cb2652321b42cc9689319075d253fe2275ace1";
    const OID: &str = "16dbe67ed4ef85711e1cd4b01848b3dd4cdd73e0";
    const PATH: &str = "server/src/owner.rs";

    #[test]
    fn baseline_relation_rejects_fake_resolution_and_non_ancestor() {
        assert_eq!(validate_baseline_relation(BASELINE, BASELINE, true), Ok(()));
        assert!(validate_baseline_relation(BASELINE, &"a".repeat(40), true).is_err());
        assert!(validate_baseline_relation(BASELINE, BASELINE, false).is_err());
    }

    #[test]
    fn tree_parser_binds_exact_path_and_blob_oid() {
        let valid = format!("100644 blob {OID}\t{PATH}\0");
        assert_eq!(parse_ls_tree_blob(valid.as_bytes(), PATH, OID), Ok(()));

        let wrong_oid = format!("100644 blob {}\t{PATH}\0", "b".repeat(40));
        assert!(parse_ls_tree_blob(wrong_oid.as_bytes(), PATH, OID).is_err());

        let wrong_path = format!("100644 blob {OID}\tserver/src/other.rs\0");
        assert!(parse_ls_tree_blob(wrong_path.as_bytes(), PATH, OID).is_err());
        assert!(
            parse_ls_tree_blob(&[valid.as_bytes(), valid.as_bytes()].concat(), PATH, OID).is_err()
        );
    }
}
