// server/src/node_agent_project_profile_python.rs

use std::path::Path;

use crate::node_agent_project_profile::ProjectProfile;

#[derive(Debug)]
pub(crate) struct PythonWorkspaceModule {
    pub(crate) module: String,
    pub(crate) manager: String,
    pub(crate) has_manage_py: bool,
    pub(crate) has_pyproject: bool,
}

pub(crate) fn detect_python_project_profile(path: &Path) -> Option<ProjectProfile> {
    let pyproject = path.join("pyproject.toml");
    let requirements = path.join("requirements.txt");
    let pytest_ini = path.join("pytest.ini");
    let manage_py = path.join("manage.py");
    let poetry_lock = path.join("poetry.lock");
    let uv_lock = path.join("uv.lock");
    let pipfile = path.join("Pipfile");

    let detected = [
        (&pyproject, "pyproject.toml"),
        (&requirements, "requirements.txt"),
        (&pytest_ini, "pytest.ini"),
        (&manage_py, "manage.py"),
        (&poetry_lock, "poetry.lock"),
        (&uv_lock, "uv.lock"),
        (&pipfile, "Pipfile"),
    ]
    .into_iter()
    .filter_map(|(path, name)| path.exists().then_some(name.to_string()))
    .collect::<Vec<_>>();
    if detected.is_empty() {
        return None;
    }

    let manager = python_package_manager(path);
    let mut profile = ProjectProfile {
        project_type: Some("Python".to_string()),
        package_manager: Some(manager.clone()),
        run_command: None,
        test_command: Some(python_tool_command(&manager, "pytest")),
        build_command: python_build_command(&manager, pyproject.exists()),
        detected_files: detected,
    };
    if manage_py.exists() {
        profile.run_command = Some(python_manage_command(&manager));
    }
    Some(profile)
}

pub(crate) fn python_workspace_commands(
    module: &PythonWorkspaceModule,
) -> (Option<String>, Option<String>, Option<String>) {
    let run_command = module
        .has_manage_py
        .then(|| python_workspace_manage_command(&module.manager, &module.module));
    let test_command = Some(python_workspace_tool_command(
        &module.manager,
        &module.module,
        "pytest",
    ));
    let build_command = module
        .has_pyproject
        .then(|| python_workspace_build_command(&module.manager, &module.module));
    (run_command, test_command, build_command)
}

fn python_package_manager(path: &Path) -> String {
    if path.join("uv.lock").exists() {
        "uv".to_string()
    } else if path.join("poetry.lock").exists() {
        "Poetry".to_string()
    } else if path.join("Pipfile").exists() {
        "Pipenv".to_string()
    } else if path.join("pyproject.toml").exists() {
        "pyproject".to_string()
    } else {
        "pip".to_string()
    }
}

fn python_tool_command(manager: &str, tool: &str) -> String {
    match manager {
        "uv" => format!("uv run {tool}"),
        "Poetry" => format!("poetry run {tool}"),
        "Pipenv" => format!("pipenv run {tool}"),
        _ => format!("python -m {tool}"),
    }
}

fn python_manage_command(manager: &str) -> String {
    match manager {
        "uv" => "uv run python manage.py runserver".to_string(),
        "Poetry" => "poetry run python manage.py runserver".to_string(),
        "Pipenv" => "pipenv run python manage.py runserver".to_string(),
        _ => "python manage.py runserver".to_string(),
    }
}

fn python_build_command(manager: &str, has_pyproject: bool) -> Option<String> {
    if !has_pyproject {
        return None;
    }
    Some(match manager {
        "uv" => "uv build".to_string(),
        "Poetry" => "poetry build".to_string(),
        "Pipenv" => "pipenv run python -m build".to_string(),
        _ => "python -m build".to_string(),
    })
}

fn python_workspace_tool_command(manager: &str, module: &str, tool: &str) -> String {
    match manager {
        "uv" => format!("uv run --project {module} {tool}"),
        "Poetry" => format!("poetry -C {module} run {tool}"),
        _ => format!("python -m {tool} {module}"),
    }
}

fn python_workspace_manage_command(manager: &str, module: &str) -> String {
    match manager {
        "uv" => format!("uv run --project {module} python {module}/manage.py runserver"),
        "Poetry" => format!("poetry -C {module} run python manage.py runserver"),
        _ => format!("python {module}/manage.py runserver"),
    }
}

fn python_workspace_build_command(manager: &str, module: &str) -> String {
    match manager {
        "uv" => format!("uv build {module}"),
        "Poetry" => format!("poetry -C {module} build"),
        _ => format!("python -m build {module}"),
    }
}


#[cfg(test)]
#[path = "node_agent_project_profile_python_tests.rs"]
mod tests;
