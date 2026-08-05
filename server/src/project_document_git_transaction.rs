//! Git-scoped before/after commits for AI document organization.

use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    process::Stdio,
};

const NON_DOCUMENT_PATHSPECS: &[&str] = &[
    ".",
    ":(exclude,icase,glob)**/*.md",
    ":(exclude,icase,glob)**/*.markdown",
    ":(exclude,icase,glob)**/*.mdx",
    ":(exclude,glob).elon/document-sections.json",
    ":(exclude,glob).elon/document-organization-suggestions.json",
    ":(exclude,glob).elon/knowledge-federation.json",
    ":(exclude,glob).elon/discussion-graph.json",
    ":(exclude,glob).elon/discussion-graph-suggestions.json",
    ":(exclude,glob).elon/project-features.json",
];

pub(crate) fn commit_document_baseline(workspace: &Path) -> Result<String> {
    scoped_document_commit(
        workspace,
        "chore(docs): snapshot before AI organization\n\nElon-Document-Phase: before",
    )
}

pub(crate) fn commit_document_result(workspace: &Path, expected_baseline: &str) -> Result<String> {
    let head = current_head(workspace)?;
    if head != expected_baseline {
        bail!(
            "文档整理期间 Git HEAD 已变化：基线 {}，当前 {}；拒绝混入其他提交",
            short_sha(expected_baseline),
            short_sha(&head)
        );
    }
    scoped_document_commit(
        workspace,
        &format!(
            "chore(docs): apply AI organization\n\nElon-Document-Phase: after\nElon-Document-Baseline: {expected_baseline}"
        ),
    )
}

pub(crate) fn current_document_head(workspace: &Path) -> Result<String> {
    current_head(workspace)
}

pub(crate) fn verify_document_baseline(
    workspace: &Path,
    baseline: &str,
    path: &str,
    expected_revision: &str,
) -> Result<()> {
    let content = git_output(
        workspace,
        &["show", &format!("{baseline}:{path}")],
        None,
        None,
    )
    .with_context(|| format!("Git 基线未包含原始文档 {path}"))?;
    let working_path = workspace.join(path);
    if working_path.is_file() {
        let working_content = fs::read(&working_path)?;
        let actual = format!("{:x}", Sha256::digest(&working_content));
        if actual != expected_revision {
            bail!("当前文档 {path} 与整理建议 source_revision 不一致");
        }
        if normalize_line_endings(&content) != normalize_line_endings(&working_content) {
            bail!("Git 基线中的 {path} 与当前原始文档不一致");
        }
    }
    Ok(())
}

fn normalize_line_endings(content: &[u8]) -> Vec<u8> {
    String::from_utf8_lossy(content)
        .replace("\r\n", "\n")
        .into_bytes()
}

fn scoped_document_commit(workspace: &Path, message: &str) -> Result<String> {
    ensure_git_workspace(workspace)?;
    let _lock = DocumentGitLock::acquire(workspace)?;
    let old_head = current_head(workspace)?;
    let index_path = git_path(workspace, "index")?;
    let original_index = fs::read(&index_path).ok();
    let staged_patch = staged_non_document_patch(workspace)?;
    let temp_index = std::env::temp_dir().join(format!(
        "elon-document-index-{}",
        uuid::Uuid::new_v4().simple()
    ));

    let result = (|| -> Result<String> {
        git_output(
            workspace,
            &["read-tree", &old_head],
            Some(&temp_index),
            None,
        )?;
        let document_paths = document_paths(workspace)?;
        if !document_paths.is_empty() {
            let pathspec_input = document_paths.join("\0") + "\0";
            git_output(
                workspace,
                &["add", "-A", "--pathspec-from-file=-", "--pathspec-file-nul"],
                Some(&temp_index),
                Some(pathspec_input.as_bytes()),
            )?;
        }
        let tree = output_text(git_output(
            workspace,
            &["write-tree"],
            Some(&temp_index),
            None,
        )?)?;
        let commit = output_text(git_output(
            workspace,
            &["commit-tree", &tree, "-p", &old_head],
            Some(&temp_index),
            Some(message.as_bytes()),
        )?)?;
        git_output(
            workspace,
            &["update-ref", "HEAD", &commit, &old_head],
            None,
            None,
        )?;
        if let Err(error) = restore_non_document_index(workspace, &staged_patch) {
            rollback_commit(
                workspace,
                &old_head,
                &commit,
                &index_path,
                original_index.as_deref(),
            );
            return Err(error);
        }
        let restored_patch = match staged_non_document_patch(workspace) {
            Ok(value) => value,
            Err(error) => {
                rollback_commit(
                    workspace,
                    &old_head,
                    &commit,
                    &index_path,
                    original_index.as_deref(),
                );
                return Err(error);
            }
        };
        if restored_patch != staged_patch {
            rollback_commit(
                workspace,
                &old_head,
                &commit,
                &index_path,
                original_index.as_deref(),
            );
            bail!("Git 文档提交未能完整保留原有非文档暂存状态，已自动回滚");
        }
        Ok(commit)
    })();

    let _ = fs::remove_file(&temp_index);
    let _ = fs::remove_file(temp_index.with_extension("lock"));
    result
}

fn restore_non_document_index(workspace: &Path, staged_patch: &[u8]) -> Result<()> {
    git_output(
        workspace,
        &["reset", "--mixed", "--quiet", "HEAD"],
        None,
        None,
    )?;
    if !staged_patch.is_empty() {
        git_output(
            workspace,
            &["apply", "--cached", "--binary", "--whitespace=nowarn", "-"],
            None,
            Some(staged_patch),
        )?;
    }
    Ok(())
}

fn rollback_commit(
    workspace: &Path,
    old_head: &str,
    new_head: &str,
    index_path: &Path,
    original_index: Option<&[u8]>,
) {
    let _ = git_output(
        workspace,
        &["update-ref", "HEAD", old_head, new_head],
        None,
        None,
    );
    match original_index {
        Some(content) => {
            let _ = fs::write(index_path, content);
        }
        None => {
            let _ = fs::remove_file(index_path);
        }
    }
}

fn staged_non_document_patch(workspace: &Path) -> Result<Vec<u8>> {
    let mut args = vec!["diff", "--cached", "--binary", "--full-index", "--"];
    args.extend_from_slice(NON_DOCUMENT_PATHSPECS);
    git_output(workspace, &args, None, None)
}

fn document_paths(workspace: &Path) -> Result<Vec<String>> {
    let output = git_output(
        workspace,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
        None,
        None,
    )?;
    let mut paths = output
        .split(|byte| *byte == 0)
        .filter(|value| !value.is_empty())
        .filter_map(|value| String::from_utf8(value.to_vec()).ok())
        .filter(|path| {
            let normalized = path.replace('\\', "/");
            let lower = normalized.to_ascii_lowercase();
            lower.ends_with(".md")
                || lower.ends_with(".markdown")
                || lower.ends_with(".mdx")
                || normalized == ".elon/document-sections.json"
                || normalized == ".elon/document-organization-suggestions.json"
                || normalized == ".elon/discussion-graph.json"
                || normalized == ".elon/discussion-graph-suggestions.json"
                || normalized == ".elon/project-features.json"
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn current_head(workspace: &Path) -> Result<String> {
    output_text(git_output(workspace, &["rev-parse", "HEAD"], None, None)?)
}

fn ensure_git_workspace(workspace: &Path) -> Result<()> {
    let inside = output_text(git_output(
        workspace,
        &["rev-parse", "--is-inside-work-tree"],
        None,
        None,
    )?)?;
    if inside != "true" {
        bail!("Git 文档事务只允许在现存工作区执行");
    }
    Ok(())
}

fn git_path(workspace: &Path, name: &str) -> Result<PathBuf> {
    let value = output_text(git_output(
        workspace,
        &["rev-parse", "--git-path", name],
        None,
        None,
    )?)?;
    let path = PathBuf::from(value);
    Ok(if path.is_absolute() {
        path
    } else {
        workspace.join(path)
    })
}

fn git_output(
    workspace: &Path,
    args: &[&str],
    index: Option<&Path>,
    stdin: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let mut command = crate::git_command_error::git_command();
    command
        .current_dir(workspace)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Yilong AI")
        .env("GIT_AUTHOR_EMAIL", "ai-docs@local")
        .env("GIT_COMMITTER_NAME", "Yilong AI")
        .env("GIT_COMMITTER_EMAIL", "ai-docs@local")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    if stdin.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = command.spawn().context("无法启动 git")?;
    if let Some(content) = stdin {
        child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("无法写入 git 标准输入"))?
            .write_all(content)?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        bail!(
            "git {} 失败：{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}

fn output_text(output: Vec<u8>) -> Result<String> {
    Ok(String::from_utf8(output)?.trim().to_string())
}

fn short_sha(value: &str) -> &str {
    value.get(..12).unwrap_or(value)
}

struct DocumentGitLock {
    path: PathBuf,
}

impl DocumentGitLock {
    fn acquire(workspace: &Path) -> Result<Self> {
        let path = git_path(workspace, "elon-document-organization.lock")?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .with_context(|| "已有文档 Git 事务正在运行")?;
        Ok(Self { path })
    }
}

impl Drop for DocumentGitLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
