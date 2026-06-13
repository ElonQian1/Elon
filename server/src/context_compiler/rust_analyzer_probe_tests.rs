use super::rust_analyzer_probe::parse_cli_findings;

#[test]
fn parses_cargo_style_diagnostic_location() {
    let output = r#"error: cannot find value `foo` in this scope
  --> src/lib.rs:12:5
   |
12 |     foo();
   |     ^^^ not found in this scope"#;

    let findings = parse_cli_findings(output, "");

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity.as_deref(), Some("error"));
    assert_eq!(findings[0].path.as_deref(), Some("src/lib.rs"));
    assert_eq!(findings[0].line, Some(12));
    assert!(findings[0].message.contains("cannot find value"));
}

#[test]
fn parses_inline_unresolved_reference_location() {
    let output = "src/main.rs:7:13: unresolved reference: MissingType";

    let findings = parse_cli_findings(output, "");

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].path.as_deref(), Some("src/main.rs"));
    assert_eq!(findings[0].line, Some(7));
    assert!(findings[0].message.contains("unresolved reference"));
}
