    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn finds_relevant_files_by_path_and_content() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "elon_context_relevance_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(dir.join("src/context")).unwrap();
        fs::write(
            dir.join("src/context/compiler.rs"),
            "pub fn build_repo_map() {}\n",
        )
        .unwrap();
        fs::write(dir.join("src/other.rs"), "pub fn unrelated() {}\n").unwrap();

        let files = find_relevant_files(&dir, "repo map compiler", 4);

        assert_eq!(files[0].path, "src/context/compiler.rs");
        assert!(files[0].score > 0);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn ignores_gitignored_relevant_files() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "elon_context_relevance_ignore_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("ignored")).unwrap();
        fs::write(dir.join(".gitignore"), "ignored/\n").unwrap();
        fs::write(dir.join("src/compiler.rs"), "pub fn build_repo_map() {}\n").unwrap();
        fs::write(
            dir.join("ignored/compiler.rs"),
            "pub fn build_repo_map_ignored() {}\n",
        )
        .unwrap();

        let files = find_relevant_files(&dir, "repo map compiler", 10);

        assert!(files.iter().any(|file| file.path == "src/compiler.rs"));
        assert!(!files.iter().any(|file| file.path == "ignored/compiler.rs"));

        fs::remove_dir_all(dir).unwrap();
    }
