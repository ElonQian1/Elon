use super::security::{invalid, valid_profile};
use super::CaptureDiagnostic;
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::Path};

const MAX_FIXTURE_BYTES: u64 = 64 * 1024;

#[derive(Debug)]
pub(super) struct PreparedFixture {
    pub(super) profile: Option<String>,
    pub(super) local_storage: BTreeMap<String, String>,
    pub(super) form_values: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureFile {
    version: u8,
    #[serde(default)]
    local_storage: BTreeMap<String, String>,
    #[serde(default)]
    form_values: BTreeMap<String, String>,
}

pub(super) fn prepare_fixture(
    root: &Path,
    profile: Option<String>,
) -> Result<PreparedFixture, CaptureDiagnostic> {
    let Some(profile) = profile else {
        return Ok(PreparedFixture {
            profile: None,
            local_storage: BTreeMap::new(),
            form_values: BTreeMap::new(),
        });
    };
    if !valid_profile(&profile) {
        return Err(invalid(
            "FIXTURE_PROFILE_INVALID",
            "fixtureProfile 只能包含 1..64 位字母、数字、下划线或连字符",
        ));
    }
    let path = root
        .join(".elon")
        .join("ui-tuner")
        .join("pwa-fixtures")
        .join(format!("{profile}.json"));
    let metadata = fs::metadata(&path).map_err(|_| {
        CaptureDiagnostic::new(
            "FIXTURE_PROFILE_NOT_PREPARED",
            "指定的 PWA fixtureProfile 不存在",
            false,
            format!(
                "创建 .elon/ui-tuner/pwa-fixtures/{profile}.json，并提交不含秘密的确定性测试数据"
            ),
        )
    })?;
    let canonical = path.canonicalize().map_err(|_| {
        invalid(
            "FIXTURE_PROFILE_PATH_REJECTED",
            "无法规范化 PWA fixtureProfile 文件路径",
        )
    })?;
    if !canonical.starts_with(root)
        || fs::symlink_metadata(&path)
            .map(|value| value.file_type().is_symlink())
            .unwrap_or(true)
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > MAX_FIXTURE_BYTES
    {
        return Err(invalid(
            "FIXTURE_PROFILE_PATH_REJECTED",
            "PWA fixtureProfile 必须是项目内 1..64KiB 的非链接普通文件",
        ));
    }
    let fixture: FixtureFile = serde_json::from_slice(
        &fs::read(canonical)
            .map_err(|_| invalid("FIXTURE_PROFILE_INVALID", "无法读取 fixtureProfile"))?,
    )
    .map_err(|_| invalid("FIXTURE_PROFILE_INVALID", "fixtureProfile JSON 无效"))?;
    if fixture.version != 1 || fixture.local_storage.len() > 64 || fixture.form_values.len() > 64 {
        return Err(invalid(
            "FIXTURE_PROFILE_INVALID",
            "fixtureProfile 必须是 version=1，且最多包含 64 个 localStorage 项和 64 个 formValues 项",
        ));
    }
    for (name, value) in &fixture.local_storage {
        let lowered = name.to_ascii_lowercase();
        if name.is_empty()
            || name.len() > 256
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
            || value.len() > 8_192
            || value.contains(['\r', '\0'])
            || [
                "token",
                "secret",
                "password",
                "passwd",
                "authorization",
                "credential",
                "session",
                "api_key",
                "apikey",
                "signature",
                "jwt",
            ]
            .iter()
            .any(|marker| lowered.contains(marker))
        {
            return Err(invalid(
                "FIXTURE_PROFILE_SECRET_REJECTED",
                "fixtureProfile 只允许非秘密确定性数据；认证材料必须使用 Windows 保护的 authProfile",
            ));
        }
    }
    for (name, value) in &fixture.form_values {
        validate_public_value(name, value)?;
    }
    Ok(PreparedFixture {
        profile: Some(profile),
        local_storage: fixture.local_storage,
        form_values: fixture.form_values,
    })
}

pub(super) fn validate_form_key(name: &str) -> Result<(), CaptureDiagnostic> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
        || secret_shaped(name)
    {
        return Err(invalid(
            "FIXTURE_FORM_KEY_REJECTED",
            "formValues key 必须是非秘密的 1..64 位稳定标识",
        ));
    }
    Ok(())
}

fn validate_public_value(name: &str, value: &str) -> Result<(), CaptureDiagnostic> {
    validate_form_key(name)?;
    if value.is_empty()
        || value.len() > 2_000
        || value.contains(['\r', '\0'])
        || secret_shaped(value)
    {
        return Err(invalid(
            "FIXTURE_PROFILE_SECRET_REJECTED",
            "formValues 只允许非秘密确定性测试值，且不得包含凭据形态或控制字符",
        ));
    }
    Ok(())
}

fn secret_shaped(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    [
        "token",
        "secret",
        "password",
        "passwd",
        "authorization",
        "credential",
        "api_key",
        "apikey",
        "signature",
        "bearer ",
        "jwt",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_fixture_rejects_secret_shaped_keys() {
        let root = std::env::temp_dir().join(format!(
            "elon-pwa-fixture-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(root.join(".elon/ui-tuner/pwa-fixtures")).unwrap();
        fs::write(
            root.join(".elon/ui-tuner/pwa-fixtures/projects.json"),
            r#"{"version":1,"localStorage":{"sessionToken":"never"}} "#,
        )
        .unwrap();
        let canonical = root.canonicalize().unwrap();
        let error = prepare_fixture(&canonical, Some("projects".into())).unwrap_err();
        assert_eq!(error.code, "FIXTURE_PROFILE_SECRET_REJECTED");
        fs::remove_dir_all(root).unwrap();
    }
}
