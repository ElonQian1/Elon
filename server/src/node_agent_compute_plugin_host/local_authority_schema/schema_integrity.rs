use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use rusqlite::Connection;
use sha2::{Digest, Sha256};

use super::{
    create_schema_objects_v3, create_schema_objects_v4, create_schema_objects_v5,
    create_schema_objects_v6, create_schema_objects_v7,
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

fn verify_schema_objects(
    connection: &Connection,
    create_reference_schema: fn(&Connection) -> Result<()>,
) -> Result<()> {
    let reference =
        Connection::open_in_memory().context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_REFERENCE_OPEN")?;
    create_reference_schema(&reference)?;

    let expected = load_schema_objects(&reference, "reference")?;
    let actual = load_schema_objects(connection, "authority")?;

    for (key, expected_fingerprint) in &expected {
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
    let mut digest = Sha256::new();
    hash_component(&mut digest, key.object_type.as_bytes());
    hash_component(&mut digest, key.name.as_bytes());
    hash_component(&mut digest, table_name.as_bytes());
    for token in tokens {
        hash_component(&mut digest, token.as_bytes());
    }
    Ok(hex::encode(digest.finalize()))
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
