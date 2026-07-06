    use super::{render_write_file_diff, write_file_diff_preview};
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn render_write_file_diff_for_existing_file() {
        let diff = render_write_file_diff("src/main.rs", Some("old\n"), "new\n").unwrap();

        assert_eq!(diff["format"], "unified");
        assert_eq!(diff["source"], "write_file");
        assert_eq!(diff["kind"], "replace");
        assert_eq!(diff["files"][0], "src/main.rs");
        assert!(diff["old_sha256"].as_str().unwrap().len() >= 64);
        assert!(diff["new_sha256"].as_str().unwrap().len() >= 64);
        assert!(diff["preview"].as_str().unwrap().contains("-old"));
        assert!(diff["preview"].as_str().unwrap().contains("+new"));
    }

    #[test]
    fn render_write_file_diff_for_new_file() {
        let diff = render_write_file_diff("docs/note.md", None, "hello\n").unwrap();

        assert_eq!(diff["kind"], "create");
        assert!(diff["old_sha256"].is_null());
        assert!(diff["preview"].as_str().unwrap().contains("--- /dev/null"));
        assert!(diff["preview"].as_str().unwrap().contains("+hello"));
    }

    #[test]
    fn render_write_file_diff_rejects_large_preview() {
        let new_content = "x\n".repeat(10_000);
        let error = render_write_file_diff("big.txt", None, &new_content).unwrap_err();

        assert!(error.to_string().contains("diff preview is too large"));
    }

    #[tokio::test]
    async fn write_file_diff_preview_reads_existing_file() {
        let temp = temp_test_dir("write_file_diff_preview_reads_existing_file");
        let path = temp.join("note.txt");
        tokio::fs::write(&path, "before\n").await.unwrap();

        let diff = write_file_diff_preview(&path, "note.txt", "after\n")
            .await
            .unwrap();

        assert_eq!(diff["kind"], "replace");
        assert!(diff["preview"].as_str().unwrap().contains("-before"));
        assert!(diff["preview"].as_str().unwrap().contains("+after"));
    }

    #[tokio::test]
    async fn write_file_diff_preview_rejects_sensitive_path_and_content() {
        let temp = temp_test_dir("write_file_diff_preview_rejects_sensitive_path_and_content");
        let env_path = temp.join(".env.local");

        let path_error = write_file_diff_preview(&env_path, ".env.local", "SAFE=value\n")
            .await
            .unwrap_err();
        assert!(path_error.to_string().contains("sensitive path"));

        let note_path = temp.join("note.txt");
        let content_error =
            write_file_diff_preview(&note_path, "note.txt", "\"api_key\": \"value\"\n")
                .await
                .unwrap_err();
        assert!(content_error.to_string().contains("sensitive new content"));

        let yaml_error = write_file_diff_preview(&note_path, "note.txt", "password: value\n")
            .await
            .unwrap_err();
        assert!(yaml_error.to_string().contains("sensitive new content"));
    }

    #[tokio::test]
    async fn write_file_diff_preview_rejects_sensitive_existing_content() {
        let temp = temp_test_dir("write_file_diff_preview_rejects_sensitive_existing_content");
        let note_path = temp.join("note.txt");
        tokio::fs::write(&note_path, "password=old-secret\n")
            .await
            .unwrap();

        let error = write_file_diff_preview(&note_path, "note.txt", "safe replacement\n")
            .await
            .unwrap_err();

        assert!(error.to_string().contains("sensitive existing content"));
    }

    #[tokio::test]
    async fn write_file_diff_preview_rejects_binary_new_content() {
        let temp = temp_test_dir("write_file_diff_preview_rejects_binary_new_content");
        let note_path = temp.join("note.txt");

        let error = write_file_diff_preview(&note_path, "note.txt", "safe\0unsafe\n")
            .await
            .unwrap_err();

        assert!(error.to_string().contains("binary new content"));
    }

    #[tokio::test]
    async fn write_file_diff_preview_rejects_oversized_new_content() {
        let temp = temp_test_dir("write_file_diff_preview_rejects_oversized_new_content");
        let note_path = temp.join("note.txt");
        let content = "x".repeat(super::MAX_WRITE_CONTENT_BYTES + 1);

        let error = write_file_diff_preview(&note_path, "note.txt", &content)
            .await
            .unwrap_err();

        assert!(error.to_string().contains("new content is too large"));
    }

    fn temp_test_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("elon-{name}-{nanos}"));
        std::fs::create_dir_all(&path).unwrap();
        path
    }
