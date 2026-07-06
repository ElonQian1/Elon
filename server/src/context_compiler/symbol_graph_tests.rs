    use super::*;

    #[test]
    fn ranks_task_symbol_and_relationships() {
        let symbols = vec![
            RustSymbol {
                id: "src/map.rs:1:struct:RepoMap".to_string(),
                name: "RepoMap".to_string(),
                kind: SymbolKind::Struct,
                path: "src/map.rs".to_string(),
                line_start: 1,
                line_end: 3,
                visibility: SymbolVisibility::Public,
                signature: "pub struct RepoMap".to_string(),
                parent: None,
                docs: None,
                role: "source",
                safety_notes: Vec::new(),
            },
            RustSymbol {
                id: "src/main.rs:4:function:build".to_string(),
                name: "build".to_string(),
                kind: SymbolKind::Function,
                path: "src/main.rs".to_string(),
                line_start: 4,
                line_end: 6,
                visibility: SymbolVisibility::Private,
                signature: "fn build(map: RepoMap)".to_string(),
                parent: None,
                docs: None,
                role: "source",
                safety_notes: Vec::new(),
            },
        ];
        let relationship = CodeRelationship {
            from_path: "src/main.rs".to_string(),
            to_symbol_id: "src/map.rs:1:struct:RepoMap".to_string(),
            to_symbol_name: "RepoMap".to_string(),
            to_path: "src/map.rs".to_string(),
            kind: RelationshipKind::TypeReference,
            line: 4,
            reason: "line mentions `RepoMap`".to_string(),
        };
        let rank = page_rank(&symbols, &[relationship.clone()]);
        let ranked = rank_symbols(
            &symbols,
            &[relationship],
            &rank,
            &["repomap".to_string()],
            8,
        );

        assert_eq!(ranked[0].name, "RepoMap");
    }
