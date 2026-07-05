use std::path::PathBuf;

pub(crate) fn resolve_structured_program(program: &str) -> PathBuf {
    #[cfg(windows)]
    {
        let trimmed = program.trim();
        if should_resolve_from_path(trimmed) {
            if let Some(path) = elon_pc_dev_runtime::command_path(trimmed) {
                return path;
            }
        }
    }
    PathBuf::from(program)
}

#[cfg(windows)]
fn should_resolve_from_path(program: &str) -> bool {
    !program.is_empty()
        && !program.contains('\\')
        && !program.contains('/')
        && !program.contains(':')
}

#[cfg(all(test, windows))]
mod tests {
    use super::resolve_structured_program;

    #[test]
    fn structured_program_resolution_does_not_choose_powershell_npm_shim() {
        let resolved = resolve_structured_program("npm");
        let lower = resolved.to_string_lossy().to_ascii_lowercase();
        assert!(!lower.ends_with(r"\npm.ps1"));
        if lower != "npm" {
            assert!(
                lower.ends_with(r"\npm.cmd")
                    || lower.ends_with(r"\npm.exe")
                    || lower.ends_with(r"\npm.bat")
                    || lower.ends_with(r"\npm.com"),
                "unexpected npm resolution: {}",
                resolved.display()
            );
        }
    }
}
