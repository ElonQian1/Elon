pub(crate) fn compatibility_fallback_args(
    extra_args: &[String],
    stdout_text: &str,
    stderr_text: &str,
) -> Option<Vec<String>> {
    let failure = format!("{stdout_text}\n{stderr_text}").to_ascii_lowercase();
    if !failure.contains("requires a newer version of codex") {
        return None;
    }
    let current_model = extra_args
        .iter()
        .find_map(|arg| arg.strip_prefix("--codex-model="));
    if current_model.is_some_and(|model| model.eq_ignore_ascii_case("gpt-5.4")) {
        return None;
    }
    let mut replaced = false;
    let mut fallback_args = extra_args
        .iter()
        .map(|arg| {
            if arg.starts_with("--codex-model=") {
                replaced = true;
                "--codex-model=gpt-5.4".to_string()
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>();
    if !replaced {
        fallback_args.push("--codex-model=gpt-5.4".to_string());
    }
    Some(fallback_args)
}

#[cfg(test)]
mod tests {
    use super::compatibility_fallback_args;

    #[test]
    fn downgrades_newer_codex_only_model_once() {
        let args = vec![
            "--codex-model=gpt-5.6-sol".to_string(),
            "--codex-effort=xhigh".to_string(),
        ];
        let fallback = compatibility_fallback_args(
            &args,
            r#"{"detail":"The 'gpt-5.6-sol' model requires a newer version of Codex."}"#,
            "",
        )
        .expect("fallback");
        assert!(fallback.contains(&"--codex-model=gpt-5.4".to_string()));
        assert!(
            compatibility_fallback_args(&fallback, "requires a newer version of Codex", "")
                .is_none()
        );
    }

    #[test]
    fn ignores_unrelated_codex_failures() {
        assert!(compatibility_fallback_args(
            &["--codex-model=gpt-5.6-sol".to_string()],
            "request timed out",
            "",
        )
        .is_none());
    }
}
