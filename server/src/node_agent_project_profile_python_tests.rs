use super::detect_python_project_profile;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_project(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "elon-python-project-profile-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn detects_plain_pyproject_python_projects() {
    let dir = temp_project("python-pyproject");
    std::fs::write(dir.join("pyproject.toml"), "[project]\nname='demo'\n").unwrap();

    let profile = detect_python_project_profile(&dir).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(profile.project_type.as_deref(), Some("Python"));
    assert_eq!(profile.package_manager.as_deref(), Some("pyproject"));
    assert_eq!(profile.test_command.as_deref(), Some("python -m pytest"));
    assert_eq!(profile.build_command.as_deref(), Some("python -m build"));
}

#[test]
fn detects_uv_django_python_projects() {
    let dir = temp_project("python-uv-django");
    std::fs::write(dir.join("pyproject.toml"), "[project]\nname='demo'\n").unwrap();
    std::fs::write(dir.join("uv.lock"), "").unwrap();
    std::fs::write(dir.join("manage.py"), "").unwrap();

    let profile = detect_python_project_profile(&dir).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(profile.package_manager.as_deref(), Some("uv"));
    assert_eq!(
        profile.run_command.as_deref(),
        Some("uv run python manage.py runserver")
    );
    assert_eq!(profile.test_command.as_deref(), Some("uv run pytest"));
    assert_eq!(profile.build_command.as_deref(), Some("uv build"));
    assert!(profile.detected_files.contains(&"manage.py".to_string()));
}

#[test]
fn detects_requirements_python_projects() {
    let dir = temp_project("python-requirements");
    std::fs::write(dir.join("requirements.txt"), "pytest\n").unwrap();
    std::fs::write(dir.join("pytest.ini"), "[pytest]\n").unwrap();

    let profile = detect_python_project_profile(&dir).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(profile.project_type.as_deref(), Some("Python"));
    assert_eq!(profile.package_manager.as_deref(), Some("pip"));
    assert_eq!(profile.test_command.as_deref(), Some("python -m pytest"));
    assert!(profile.build_command.is_none());
    assert!(profile
        .detected_files
        .contains(&"requirements.txt".to_string()));
}
