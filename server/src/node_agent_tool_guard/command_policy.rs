use anyhow::Result;
use std::path::Path;
use tokio::process::Command;

pub(crate) fn command_allowed(command: &str) -> bool {
    let lower = command.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    let shell_markers = [";", "&&", "||", "|", "\n", "\r", ">", "<", "$", "`"];
    if shell_markers.iter().any(|marker| lower.contains(marker)) {
        return false;
    }
    if contains_absolute_path_argument(&lower) {
        return false;
    }
    let blocked = [
        "remove-item",
        "del ",
        " del ",
        "rmdir ",
        " rmdir ",
        "format ",
        "shutdown",
        "restart-computer",
        "set-executionpolicy",
        "reg delete",
        "sc delete",
        "takeown",
        "icacls",
        "invoke-webrequest",
        " iwr ",
        "curl ",
        "invoke-expression",
        "start-process",
        "powershell",
        "pwsh",
        "cmd ",
        "cmd.exe",
    ];
    if blocked.iter().any(|pattern| lower.contains(pattern)) {
        return false;
    }
    if legacy_git_push_has_high_risk_args(&lower) {
        return false;
    }
    if lower.starts_with("git rebase") {
        return legacy_git_rebase_allowed(&lower);
    }
    let allowed_prefixes = [
        "git status",
        "git diff",
        "git log",
        "git show",
        "git branch",
        "git remote",
        "git fetch",
        "git pull --ff-only",
        "git add",
        "git commit",
        "git push",
        "cargo check",
        "cargo test",
        "cargo build",
        "cargo fmt",
        "cargo clippy",
        "cargo run",
        "rustfmt ",
        "npm test",
        "npm run lint",
        "npm run test",
        "npm run build",
        "npm run check",
        "npm run format",
        "npm run typecheck",
        "pnpm test",
        "pnpm run lint",
        "pnpm run test",
        "pnpm run build",
        "pnpm run check",
        "pnpm run format",
        "pnpm run typecheck",
        "yarn test",
        "yarn run lint",
        "yarn run test",
        "yarn run build",
        "yarn run check",
        "yarn run format",
        "yarn run typecheck",
        "bun test",
        "bun run lint",
        "bun run test",
        "bun run build",
        "bun run check",
        "python -m pytest",
        "python -m unittest",
        "pytest",
        "go test",
        "go vet",
        "go build",
        "dotnet test",
        "dotnet build",
        ".\\gradlew.bat test",
        ".\\gradlew.bat :app:assembledebug",
        ".\\gradlew.bat testdebugunittest",
        "gradle test",
        "gradle build",
    ];
    allowed_prefixes
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

pub(crate) fn structured_command_allowed(program: &str, args: &[String]) -> bool {
    let program = program.trim().to_ascii_lowercase();
    if !program_name_allowed(&program) || args.iter().any(|arg| !command_arg_safe(arg)) {
        return false;
    }

    match program.as_str() {
        "git" => git_args_allowed(args),
        "cargo" => first_arg_in(args, &["check", "test", "build", "fmt", "clippy", "run"]),
        "rustfmt" => !args.is_empty(),
        "npm" => package_manager_args_allowed(args, false),
        "pnpm" | "yarn" | "bun" => package_manager_args_allowed(args, true),
        "python" => {
            args.len() >= 2 && args[0] == "-m" && matches!(args[1].as_str(), "pytest" | "unittest")
        }
        "pytest" => true,
        "go" => first_arg_in(args, &["test", "vet", "build"]),
        "dotnet" => first_arg_in(args, &["test", "build"]),
        "gradle" | ".\\gradlew.bat" | "./gradlew" | "./gradlew.bat" | "gradlew.bat" => {
            first_arg_in(
                args,
                &["test", "build", "testDebugUnitTest", ":app:assembleDebug"],
            )
        }
        _ => false,
    }
}

pub(crate) fn program_name_allowed(program: &str) -> bool {
    matches!(
        program,
        "git"
            | "cargo"
            | "rustfmt"
            | "npm"
            | "pnpm"
            | "yarn"
            | "bun"
            | "python"
            | "pytest"
            | "go"
            | "dotnet"
            | "gradle"
            | ".\\gradlew.bat"
            | "./gradlew"
            | "./gradlew.bat"
            | "gradlew.bat"
    )
}

pub(crate) fn first_arg_in(args: &[String], allowed: &[&str]) -> bool {
    args.first()
        .is_some_and(|arg| allowed.iter().any(|item| arg == item))
}

pub(crate) fn git_args_allowed(args: &[String]) -> bool {
    let Some(first) = args.first().map(String::as_str) else {
        return false;
    };
    match first {
        "status" | "diff" | "log" | "show" | "branch" | "remote" | "fetch" | "add" | "commit"
        | "push" => first != "push" || !git_push_args_high_risk(args),
        "pull" => args.iter().any(|arg| arg == "--ff-only"),
        "rebase" => git_rebase_args_allowed(args),
        _ => false,
    }
}

pub(crate) fn legacy_git_rebase_allowed(command: &str) -> bool {
    let parts: Vec<&str> = command.split_whitespace().collect();
    matches!(
        parts.as_slice(),
        ["git", "rebase", "origin/main"] | ["git", "rebase", "--continue"]
    )
}

pub(crate) fn git_rebase_args_allowed(args: &[String]) -> bool {
    args.len() == 2 && matches!(args[1].as_str(), "origin/main" | "--continue")
}

pub(crate) fn legacy_git_push_has_high_risk_args(command: &str) -> bool {
    let mut parts = command.split_whitespace();
    if parts.next() != Some("git") || parts.next() != Some("push") {
        return false;
    }
    parts.any(high_risk_git_push_arg)
}

pub(crate) fn git_push_args_high_risk(args: &[String]) -> bool {
    args.iter()
        .skip(1)
        .any(|arg| high_risk_git_push_arg(arg.as_str()))
}

pub(crate) fn high_risk_git_push_arg(arg: &str) -> bool {
    let lower = arg.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return true;
    }
    matches!(
        lower.as_str(),
        "-f" | "-d" | "--delete" | "--mirror" | "--all" | "--tags" | "--prune"
    ) || lower.starts_with("--force")
        || lower.starts_with('+')
        || lower.starts_with(':')
}

pub(crate) fn package_manager_args_allowed(args: &[String], run_required: bool) -> bool {
    let Some(first) = args.first().map(String::as_str) else {
        return false;
    };
    if first == "test" && !run_required {
        return true;
    }
    first == "run"
        && args
            .get(1)
            .is_some_and(|script| allowed_package_script(script))
}

pub(crate) fn allowed_package_script(script: &str) -> bool {
    matches!(
        script,
        "lint" | "test" | "build" | "check" | "format" | "typecheck"
    )
}

pub(crate) fn command_arg_safe(arg: &str) -> bool {
    let lower = arg.trim().to_ascii_lowercase();
    if lower.is_empty() || lower.contains('\0') {
        return false;
    }
    let shell_markers = [";", "&&", "||", "|", "\n", "\r", ">", "<", "$", "`"];
    if shell_markers.iter().any(|marker| lower.contains(marker)) {
        return false;
    }
    if contains_absolute_path_argument(&lower) {
        return false;
    }
    let path_like = lower.contains('/') || lower.contains('\\');
    if path_like {
        let normalized = lower.replace('\\', "/");
        if normalized
            .split('/')
            .any(|part| part == ".." || part == ".git")
        {
            return false;
        }
    }
    true
}

pub(crate) fn contains_absolute_path_argument(command: &str) -> bool {
    let bytes = command.as_bytes();
    if bytes.starts_with(b"\\\\") {
        return true;
    }
    for index in 0..bytes.len().saturating_sub(2) {
        let drive = bytes[index];
        if !drive.is_ascii_alphabetic() || bytes[index + 1] != b':' {
            continue;
        }
        if bytes[index + 2] != b'\\' && bytes[index + 2] != b'/' {
            continue;
        }
        if index == 0 || bytes[index - 1].is_ascii_whitespace() || bytes[index - 1] == b'"' {
            return true;
        }
    }
    command.contains(" \\\\")
}
