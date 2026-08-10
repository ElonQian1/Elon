use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use rusqlite::Connection;
use sha2::{Digest, Sha256};

use super::{
    create_schema_objects_v3, create_schema_objects_v4, create_schema_objects_v5,
    create_schema_objects_v6, create_schema_objects_v7, work_admission,
};

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
struct SchemaObjectKey {
    object_type: String,
    name: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct SchemaObjectFingerprint {
    table_name: String,
    definition_sha256: String,
}

pub(super) fn verify_schema_objects_v3(connection: &Connection) -> Result<()> {
    verify_schema_objects(connection, create_schema_objects_v3)
}

pub(super) fn verify_schema_objects_v4(connection: &Connection) -> Result<()> {
    verify_schema_objects(connection, create_schema_objects_v4)
}

pub(super) fn verify_schema_objects_v5(connection: &Connection) -> Result<()> {
    verify_schema_objects(connection, create_schema_objects_v5)
}

pub(super) fn verify_schema_objects_v6(connection: &Connection) -> Result<()> {
    verify_schema_objects(connection, create_schema_objects_v6)
}

pub(super) fn verify_schema_objects_v7(connection: &Connection) -> Result<()> {
    verify_schema_objects(connection, create_schema_objects_v7)
}

pub(super) fn verify_schema_objects_v8(connection: &Connection) -> Result<()> {
    verify_schema_objects(connection, create_schema_objects_v8_reference)
}

fn create_schema_objects_v8_reference(connection: &Connection) -> Result<()> {
    create_schema_objects_v7(connection)?;
    work_admission::create_schema_objects_v8(connection)
}

fn verify_schema_objects(
    connection: &Connection,
    create_reference_schema: fn(&Connection) -> Result<()>,
) -> Result<()> {
    let reference =
        Connection::open_in_memory().context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_REFERENCE_OPEN")?;
    create_reference_schema(&reference)?;

    let expected = load_schema_objects(&reference, "reference")?;
    let actual = load_schema_objects(connection, "authority")?;
    compare_schema_objects(&expected, &actual)
}

/// Verifies one already-opened authority using only source-frozen DDL tokens and reads from that
/// same connection. Unlike the migration verifier above, this never opens a reference database or
/// executes schema SQL and is therefore safe inside the planning query-only transaction.
pub(super) fn verify_schema_objects_from_definitions<'a>(
    connection: &Connection,
    definition_batches: impl IntoIterator<Item = &'a str>,
) -> Result<()> {
    let expected = load_schema_objects_from_definitions(definition_batches)?;
    let actual = load_schema_objects(connection, "authority")?;
    compare_schema_objects(&expected, &actual)
}

fn compare_schema_objects(
    expected: &BTreeMap<SchemaObjectKey, SchemaObjectFingerprint>,
    actual: &BTreeMap<SchemaObjectKey, SchemaObjectFingerprint>,
) -> Result<()> {
    for (key, expected_fingerprint) in expected {
        let Some(actual_fingerprint) = actual.get(key) else {
            bail!(
                "COMPUTE_PLUGIN_AUTHORITY_SCHEMA_INCOMPLETE: missing {} {}",
                key.object_type,
                key.name
            );
        };
        if actual_fingerprint != expected_fingerprint {
            bail!(
                "COMPUTE_PLUGIN_AUTHORITY_SCHEMA_DEFINITION_MISMATCH: {} {} expected_table={} actual_table={} expected_sha256={} actual_sha256={}",
                key.object_type,
                key.name,
                expected_fingerprint.table_name,
                actual_fingerprint.table_name,
                expected_fingerprint.definition_sha256,
                actual_fingerprint.definition_sha256
            );
        }
    }

    for key in actual.keys() {
        if !expected.contains_key(key) {
            bail!(
                "COMPUTE_PLUGIN_AUTHORITY_SCHEMA_UNEXPECTED: unexpected {} {}",
                key.object_type,
                key.name
            );
        }
    }

    Ok(())
}

fn load_schema_objects_from_definitions<'a>(
    definition_batches: impl IntoIterator<Item = &'a str>,
) -> Result<BTreeMap<SchemaObjectKey, SchemaObjectFingerprint>> {
    let mut objects = BTreeMap::new();
    for batch in definition_batches {
        for tokens in split_schema_statements(tokenize_sql(batch)?)? {
            apply_schema_statement(&mut objects, &tokens)?;
        }
    }
    Ok(objects)
}

fn split_schema_statements(tokens: Vec<String>) -> Result<Vec<Vec<String>>> {
    let mut statements = Vec::new();
    let mut start = 0;
    for (index, token) in tokens.iter().enumerate() {
        if token != ";" {
            continue;
        }
        let next = tokens.get(index + 1).map(String::as_str);
        if next.is_none() || next.is_some_and(|value| matches!(value, "CREATE" | "DROP")) {
            if start == index {
                bail!("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_SOURCE_EMPTY_STATEMENT");
            }
            statements.push(tokens[start..index].to_vec());
            start = index + 1;
        }
    }
    if start < tokens.len() {
        statements.push(tokens[start..].to_vec());
    }
    if statements.is_empty() {
        bail!("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_SOURCE_EMPTY");
    }
    Ok(statements)
}

fn apply_schema_statement(
    objects: &mut BTreeMap<SchemaObjectKey, SchemaObjectFingerprint>,
    tokens: &[String],
) -> Result<()> {
    match tokens.first().map(String::as_str) {
        Some("CREATE") => insert_schema_statement(objects, tokens),
        Some("DROP") => drop_schema_statement(objects, tokens),
        _ => bail!("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_SOURCE_STATEMENT_INVALID"),
    }
}

fn insert_schema_statement(
    objects: &mut BTreeMap<SchemaObjectKey, SchemaObjectFingerprint>,
    tokens: &[String],
) -> Result<()> {
    let object_index = if tokens.get(1).map(String::as_str) == Some("UNIQUE") {
        2
    } else {
        1
    };
    let object_type = tokens
        .get(object_index)
        .map(|value| value.to_ascii_lowercase())
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_SOURCE_TYPE_MISSING")?;
    let name = tokens
        .get(object_index + 1)
        .cloned()
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_SOURCE_NAME_MISSING")?;
    if !matches!(object_type.as_str(), "table" | "index" | "trigger") {
        bail!("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_SOURCE_TYPE_INVALID");
    }
    let table_name = if object_type == "table" {
        name.clone()
    } else {
        let on_index = tokens
            .iter()
            .enumerate()
            .skip(object_index + 2)
            .find_map(|(index, token)| (token == "ON").then_some(index))
            .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_SOURCE_TABLE_MISSING")?;
        tokens
            .get(on_index + 1)
            .cloned()
            .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_SOURCE_TABLE_MISSING")?
    };
    let key = SchemaObjectKey { object_type, name };
    let fingerprint = SchemaObjectFingerprint {
        table_name: table_name.clone(),
        definition_sha256: definition_digest_from_tokens(&key, &table_name, tokens),
    };
    if objects.insert(key, fingerprint).is_some() {
        bail!("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_SOURCE_DUPLICATE");
    }
    Ok(())
}

fn drop_schema_statement(
    objects: &mut BTreeMap<SchemaObjectKey, SchemaObjectFingerprint>,
    tokens: &[String],
) -> Result<()> {
    let key = SchemaObjectKey {
        object_type: tokens
            .get(1)
            .map(|value| value.to_ascii_lowercase())
            .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_SOURCE_DROP_TYPE_MISSING")?,
        name: tokens
            .get(2)
            .cloned()
            .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_SOURCE_DROP_NAME_MISSING")?,
    };
    if objects.remove(&key).is_none() {
        bail!("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_SOURCE_DROP_MISSING");
    }
    Ok(())
}

fn load_schema_objects(
    connection: &Connection,
    source: &str,
) -> Result<BTreeMap<SchemaObjectKey, SchemaObjectFingerprint>> {
    let mut statement = connection
        .prepare(
            "SELECT type, name, tbl_name, sql
             FROM sqlite_master
             WHERE type IN ('table', 'index', 'trigger')
               AND name NOT GLOB 'sqlite_*'
             ORDER BY type, name",
        )
        .with_context(|| format!("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_OBJECTS_PREPARE: {source}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .with_context(|| format!("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_OBJECTS_QUERY: {source}"))?;

    let mut objects = BTreeMap::new();
    for row in rows {
        let (object_type, name, table_name, definition) =
            row.with_context(|| format!("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_OBJECTS_READ: {source}"))?;
        let definition = definition.with_context(|| {
            format!(
                "COMPUTE_PLUGIN_AUTHORITY_SCHEMA_DEFINITION_MISSING: {source} {object_type} {name}"
            )
        })?;
        let key = SchemaObjectKey { object_type, name };
        let definition_sha256 = definition_digest(&key, &table_name, &definition)?;
        let fingerprint = SchemaObjectFingerprint {
            table_name,
            definition_sha256,
        };
        if objects.insert(key.clone(), fingerprint).is_some() {
            bail!(
                "COMPUTE_PLUGIN_AUTHORITY_SCHEMA_OBJECT_DUPLICATE: {source} {} {}",
                key.object_type,
                key.name
            );
        }
    }
    Ok(objects)
}

fn definition_digest(key: &SchemaObjectKey, table_name: &str, definition: &str) -> Result<String> {
    let tokens = tokenize_sql(definition).with_context(|| {
        format!(
            "COMPUTE_PLUGIN_AUTHORITY_SCHEMA_DEFINITION_PARSE: {} {}",
            key.object_type, key.name
        )
    })?;
    Ok(definition_digest_from_tokens(key, table_name, &tokens))
}

fn definition_digest_from_tokens(
    key: &SchemaObjectKey,
    table_name: &str,
    tokens: &[String],
) -> String {
    let mut digest = Sha256::new();
    hash_component(&mut digest, key.object_type.as_bytes());
    hash_component(&mut digest, key.name.as_bytes());
    hash_component(&mut digest, table_name.as_bytes());
    for token in tokens {
        hash_component(&mut digest, token.as_bytes());
    }
    hex::encode(digest.finalize())
}

fn hash_component(digest: &mut Sha256, value: &[u8]) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value);
}

fn tokenize_sql(sql: &str) -> Result<Vec<String>> {
    let characters = sql.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut cursor = 0;

    while cursor < characters.len() {
        let current = characters[cursor];
        if current.is_whitespace() {
            cursor += 1;
            continue;
        }
        if current == '-' && characters.get(cursor + 1) == Some(&'-') {
            cursor += 2;
            while cursor < characters.len() && characters[cursor] != '\n' {
                cursor += 1;
            }
            continue;
        }
        if current == '/' && characters.get(cursor + 1) == Some(&'*') {
            cursor += 2;
            let mut closed = false;
            while cursor + 1 < characters.len() {
                if characters[cursor] == '*' && characters[cursor + 1] == '/' {
                    cursor += 2;
                    closed = true;
                    break;
                }
                cursor += 1;
            }
            if !closed {
                bail!("unterminated block comment");
            }
            continue;
        }
        if matches!(current, '\'' | '"' | '`') {
            let (token, next_cursor) = quoted_token(&characters, cursor, current)?;
            tokens.push(token);
            cursor = next_cursor;
            continue;
        }
        if current == '[' {
            let (token, next_cursor) = bracketed_token(&characters, cursor)?;
            tokens.push(token);
            cursor = next_cursor;
            continue;
        }
        if is_word_character(current) {
            let start = cursor;
            cursor += 1;
            while cursor < characters.len() && is_word_character(characters[cursor]) {
                cursor += 1;
            }
            tokens.push(characters[start..cursor].iter().collect());
            continue;
        }

        tokens.push(current.to_string());
        cursor += 1;
    }

    Ok(tokens)
}

fn quoted_token(characters: &[char], start: usize, quote: char) -> Result<(String, usize)> {
    let mut cursor = start + 1;
    while cursor < characters.len() {
        if characters[cursor] == quote {
            if characters.get(cursor + 1) == Some(&quote) {
                cursor += 2;
                continue;
            }
            cursor += 1;
            return Ok((characters[start..cursor].iter().collect(), cursor));
        }
        cursor += 1;
    }
    bail!("unterminated quoted token")
}

fn bracketed_token(characters: &[char], start: usize) -> Result<(String, usize)> {
    let mut cursor = start + 1;
    while cursor < characters.len() {
        if characters[cursor] == ']' {
            cursor += 1;
            return Ok((characters[start..cursor].iter().collect(), cursor));
        }
        cursor += 1;
    }
    bail!("unterminated bracketed identifier")
}

fn is_word_character(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '_' | '$') || !character.is_ascii()
}
