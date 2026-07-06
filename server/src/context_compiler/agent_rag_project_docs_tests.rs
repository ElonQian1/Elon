    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn loads_existing_ai_project_docs_into_pack_section() {
        let workspace = temp_dir("elon_agent_project_docs");
        fs::create_dir_all(&workspace).unwrap();
        fs::write(
            workspace.join("AI_PROJECT.md"),
            "# Demo\n\nProject overview.\n",
        )
        .unwrap();
        fs::write(workspace.join("AI_RULES.md"), "# Rules\n\nRun tests.\n").unwrap();

        let context = load_agent_project_docs(&workspace);

        assert_eq!(context.included_count, 2);
        assert_eq!(context.missing_count, PROJECT_DOC_SPECS.len() - 2);
        assert!(context.pack_section.contains("<project_docs_context"));
        assert!(context.pack_section.contains("AI_PROJECT.md"));
        assert!(context.pack_section.contains("Project overview."));
        assert!(context.pack_section.contains("AI_RULES.md"));

        fs::remove_dir_all(workspace).ok();
    }

    #[test]
    fn truncates_large_project_docs_with_bounded_budget() {
        let workspace = temp_dir("elon_agent_project_docs_large");
        fs::create_dir_all(&workspace).unwrap();
        fs::write(workspace.join("AI_PROJECT.md"), "A".repeat(10_000)).unwrap();

        let context = load_agent_project_docs(&workspace);
        let doc = context
            .documents
            .iter()
            .find(|doc| doc.path == "AI_PROJECT.md")
            .expect("AI_PROJECT doc");

        assert!(context.truncated);
        assert!(doc.truncated);
        assert!(doc.snippet_char_count <= 2_400);
        assert!(context.pack_section.contains("project doc truncated"));

        fs::remove_dir_all(workspace).ok();
    }

    #[test]
    fn prepends_project_docs_before_symbol_pack() {
        let context = AgentProjectDocsContext {
            total_budget_chars: PROJECT_DOCS_TOTAL_BUDGET,
            included_count: 1,
            missing_count: 0,
            truncated: false,
            documents: Vec::new(),
            pack_section: "<project_docs_context>\n</project_docs_context>\n".to_string(),
        };

        let pack = prepend_project_docs_to_pack(&context, "<symbol_task_context />");

        assert!(pack.starts_with("<project_docs_context>"));
        assert!(pack.contains("<symbol_task_context />"));
    }

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), nonce))
    }
