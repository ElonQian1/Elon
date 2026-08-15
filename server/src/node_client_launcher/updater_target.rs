use super::*;

pub(super) const EXPECTED_RELEASE_IDENTITY_ENV: &str = "ELON_EXPECTED_UPDATE_RELEASE_IDENTITY";

pub(super) fn verify_expected_from_env(remote: &VersionInfo) -> Result<()> {
    verify_expected(
        remote,
        std::env::var(EXPECTED_RELEASE_IDENTITY_ENV).ok().as_deref(),
    )
}

fn verify_expected(remote: &VersionInfo, expected: Option<&str>) -> Result<()> {
    let Some(expected) = expected.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    anyhow::ensure!(
        valid_release_identity(expected),
        "AI 更新重启请求携带的精确发布身份无效；拒绝下载或替换"
    );
    let actual = format!("{}+{}", remote.version.trim(), remote.git_sha.trim());
    anyhow::ensure!(
        valid_release_identity(&actual),
        "服务器版本清单缺少可验证的 version+gitSha；拒绝执行 AI 更新重启"
    );
    anyhow::ensure!(
        actual.eq_ignore_ascii_case(expected),
        "服务器当前 Win 发布身份 {actual} 与 AI 请求的精确目标 {expected} 不一致；拒绝替换"
    );
    Ok(())
}

fn valid_release_identity(value: &str) -> bool {
    let Some((version, git_sha)) = value.rsplit_once('+') else {
        return false;
    };
    !version.is_empty()
        && version.len() <= 48
        && version
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
        && (40..=64).contains(&git_sha.len())
        && git_sha.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote(version: &str, git_sha: &str) -> VersionInfo {
        VersionInfo {
            version: version.to_string(),
            git_sha: git_sha.to_string(),
            download_url: String::new(),
            windows_client_download_url: String::new(),
            download_sha256: String::new(),
            file_sha256: String::new(),
            windows_client_sha256: String::new(),
        }
    }

    #[test]
    fn update_restart_exact_target_accepts_only_the_requested_release() {
        let sha = "a".repeat(40);
        let version = remote("0.3.69", &sha);
        assert!(verify_expected(&version, Some(&format!("0.3.69+{sha}"))).is_ok());
        assert!(verify_expected(&version, None).is_ok());
        assert!(verify_expected(&version, Some(&format!("0.3.70+{sha}"))).is_err());
        assert!(verify_expected(&version, Some("latest")).is_err());
    }
}
