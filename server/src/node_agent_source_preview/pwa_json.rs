use super::{
    pwa_writer::{conflict, invalid, PwaCommitError},
    types::PwaExplicitStyleBinding,
};
use std::collections::BTreeMap;

pub(super) fn edit_token_json(
    content: &str,
    binding: &PwaExplicitStyleBinding,
    changes: &BTreeMap<String, String>,
) -> Result<String, PwaCommitError> {
    serde_json::from_str::<serde_json::Value>(content)
        .map_err(|error| invalid(format!("token-json 源文件不是有效 JSON: {error}")))?;
    let path = json_target_path(&binding.target)?;
    let expected = find_json_path(content, 0, &path)?
        .ok_or_else(|| conflict("token-json target 在源文件中不存在"))?;
    if expected != (binding.range.start, binding.range.end) {
        return Err(conflict("token-json target 与 range 锚点不匹配"));
    }
    let snippet = &content[expected.0..expected.1];
    let mut value = serde_json::from_str::<serde_json::Value>(snippet)
        .map_err(|error| conflict(format!("token-json range 锚点已失效: {error}")))?;
    if !value.is_object() {
        return Err(conflict("token-json target 必须指向 JSON object"));
    }
    for (property, next) in changes {
        set_json_property(&mut value, property, next)?;
    }
    let serialized = serde_json::to_string_pretty(&value)
        .map_err(|error| invalid(format!("无法序列化 token-json: {error}")))?;
    Ok(indent_multiline(
        &serialized,
        line_indentation(content, binding.range.start),
    ))
}

fn set_json_property(
    root: &mut serde_json::Value,
    property: &str,
    next: &str,
) -> Result<(), PwaCommitError> {
    let segments = property.split('.').collect::<Vec<_>>();
    let mut current = root;
    for segment in &segments[..segments.len() - 1] {
        current = current
            .as_object_mut()
            .and_then(|object| object.get_mut(*segment))
            .ok_or_else(|| conflict(format!("token-json 属性路径锚点不存在: {property}")))?;
    }
    let object = current
        .as_object_mut()
        .ok_or_else(|| conflict(format!("token-json 属性路径不是 object: {property}")))?;
    object.insert(
        segments[segments.len() - 1].to_string(),
        serde_json::Value::String(next.to_string()),
    );
    Ok(())
}

fn json_target_path(target: &str) -> Result<Vec<String>, PwaCommitError> {
    if target == "$" {
        return Ok(Vec::new());
    }
    let segments = if let Some(pointer) = target.strip_prefix('/') {
        pointer
            .split('/')
            .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
            .collect::<Vec<_>>()
    } else {
        target
            .strip_prefix("$.")
            .unwrap_or(target)
            .split('.')
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    if segments.is_empty() || segments.iter().any(|segment| segment.is_empty()) {
        return Err(invalid(
            "token-json target 必须是 $、JSON Pointer 或点分路径",
        ));
    }
    Ok(segments)
}

fn find_json_path(
    input: &str,
    start: usize,
    path: &[String],
) -> Result<Option<(usize, usize)>, PwaCommitError> {
    let bytes = input.as_bytes();
    let value_start = skip_json_ws(bytes, start);
    if path.is_empty() {
        return Ok(Some((value_start, skip_json_value(input, value_start)?)));
    }
    if bytes.get(value_start) != Some(&b'{') {
        return Ok(None);
    }
    let mut index = skip_json_ws(bytes, value_start + 1);
    while bytes.get(index) != Some(&b'}') {
        if bytes.get(index) != Some(&b'"') {
            return Err(invalid("token-json object key 语法无效"));
        }
        let key_end = skip_json_string(bytes, index)?;
        let key = serde_json::from_str::<String>(&input[index..key_end])
            .map_err(|error| invalid(format!("token-json key 无效: {error}")))?;
        index = skip_json_ws(bytes, key_end);
        if bytes.get(index) != Some(&b':') {
            return Err(invalid("token-json object key 后缺少冒号"));
        }
        let child_start = skip_json_ws(bytes, index + 1);
        if key == path[0] {
            return find_json_path(input, child_start, &path[1..]);
        }
        index = skip_json_ws(bytes, skip_json_value(input, child_start)?);
        match bytes.get(index) {
            Some(b',') => index = skip_json_ws(bytes, index + 1),
            Some(b'}') => break,
            _ => return Err(invalid("token-json object 分隔符无效")),
        }
    }
    Ok(None)
}

fn skip_json_value(input: &str, start: usize) -> Result<usize, PwaCommitError> {
    let bytes = input.as_bytes();
    match bytes.get(start) {
        Some(b'"') => skip_json_string(bytes, start),
        Some(b'{') => {
            let mut index = skip_json_ws(bytes, start + 1);
            if bytes.get(index) == Some(&b'}') {
                return Ok(index + 1);
            }
            loop {
                if bytes.get(index) != Some(&b'"') {
                    return Err(invalid("token-json object key 语法无效"));
                }
                index = skip_json_ws(bytes, skip_json_string(bytes, index)?);
                if bytes.get(index) != Some(&b':') {
                    return Err(invalid("token-json object key 后缺少冒号"));
                }
                index = skip_json_ws(
                    bytes,
                    skip_json_value(input, skip_json_ws(bytes, index + 1))?,
                );
                match bytes.get(index) {
                    Some(b',') => index = skip_json_ws(bytes, index + 1),
                    Some(b'}') => return Ok(index + 1),
                    _ => return Err(invalid("token-json object 分隔符无效")),
                }
            }
        }
        Some(b'[') => {
            let mut index = skip_json_ws(bytes, start + 1);
            if bytes.get(index) == Some(&b']') {
                return Ok(index + 1);
            }
            loop {
                index = skip_json_ws(bytes, skip_json_value(input, index)?);
                match bytes.get(index) {
                    Some(b',') => index = skip_json_ws(bytes, index + 1),
                    Some(b']') => return Ok(index + 1),
                    _ => return Err(invalid("token-json array 分隔符无效")),
                }
            }
        }
        Some(_) => {
            let mut index = start;
            while let Some(byte) = bytes.get(index) {
                if byte.is_ascii_whitespace() || matches!(byte, b',' | b']' | b'}') {
                    break;
                }
                index += 1;
            }
            if index == start {
                Err(invalid("token-json value 语法无效"))
            } else {
                Ok(index)
            }
        }
        None => Err(invalid("token-json value 越出文件范围")),
    }
}

fn skip_json_string(bytes: &[u8], start: usize) -> Result<usize, PwaCommitError> {
    let mut index = start + 1;
    let mut escaped = false;
    while let Some(byte) = bytes.get(index) {
        if escaped {
            escaped = false;
        } else if *byte == b'\\' {
            escaped = true;
        } else if *byte == b'"' {
            return Ok(index + 1);
        }
        index += 1;
    }
    Err(invalid("token-json 字符串未闭合"))
}

fn skip_json_ws(bytes: &[u8], mut index: usize) -> usize {
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        index += 1;
    }
    index
}

fn line_indentation(content: &str, position: usize) -> &str {
    let line_start = content[..position].rfind('\n').map_or(0, |index| index + 1);
    let line = &content[line_start..position];
    &line[..line.len() - line.trim_start().len()]
}

fn indent_multiline(value: &str, indentation: &str) -> String {
    value.replace('\n', &format!("\n{indentation}"))
}
