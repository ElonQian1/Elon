use super::{
    pwa_writer::{conflict, invalid, PwaCommitError},
    types::PwaExplicitStyleBinding,
};
use std::collections::BTreeMap;

pub(super) fn edit_css_rule(
    content: &str,
    binding: &PwaExplicitStyleBinding,
    changes: &BTreeMap<String, String>,
) -> Result<String, PwaCommitError> {
    let snippet = &content[binding.range.start..binding.range.end];
    let (open, close) = outer_braces(snippet)?;
    if snippet[..open].trim() != binding.target {
        return Err(conflict("css-rule target 与 range 锚点不匹配"));
    }
    for (property, value) in changes {
        validate_css_value(property, value)?;
    }
    edit_declaration_body(snippet, open, close, changes, DeclarationKind::Css)
}

pub(super) fn edit_style_object(
    content: &str,
    binding: &PwaExplicitStyleBinding,
    changes: &BTreeMap<String, String>,
) -> Result<String, PwaCommitError> {
    let snippet = &content[binding.range.start..binding.range.end];
    let (open, close) = outer_braces(snippet)?;
    let header = snippet[..open].trim();
    let preceding = utf8_tail(&content[..binding.range.start], 512);
    if !style_anchor_matches(header, preceding, &binding.target) {
        return Err(conflict("style-object target 与 range 锚点不匹配"));
    }
    edit_declaration_body(snippet, open, close, changes, DeclarationKind::StyleObject)
}

#[derive(Clone, Copy)]
enum DeclarationKind {
    Css,
    StyleObject,
}

fn edit_declaration_body(
    snippet: &str,
    open: usize,
    close: usize,
    changes: &BTreeMap<String, String>,
    kind: DeclarationKind,
) -> Result<String, PwaCommitError> {
    let body_start = open + 1;
    let body = &snippet[body_start..close];
    let separator = match kind {
        DeclarationKind::Css => b';',
        DeclarationKind::StyleObject => b',',
    };
    let mut entries: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for (start, end) in split_top_level(body, separator)? {
        let segment = &body[start..end];
        let Some(colon) = find_top_level(segment, b':')? else {
            continue;
        };
        let Some(key) = declaration_key(&segment[..colon], kind) else {
            continue;
        };
        let (value_start, value_end) = trimmed_bounds(segment, colon + 1, segment.len());
        if value_start == value_end {
            return Err(conflict(format!("锚点属性 {key} 没有可替换的值")));
        }
        if entries
            .insert(
                key.clone(),
                (
                    body_start + start + value_start,
                    body_start + start + value_end,
                ),
            )
            .is_some()
        {
            return Err(conflict(format!("锚点内属性 {key} 重复，拒绝猜测写回位置")));
        }
    }
    let mut edits = Vec::new();
    let mut missing = Vec::new();
    for (property, value) in changes {
        let replacement = match kind {
            DeclarationKind::Css => value.clone(),
            DeclarationKind::StyleObject => serde_json::to_string(value)
                .map_err(|error| invalid(format!("无法序列化样式值: {error}")))?,
        };
        if let Some((start, end)) = entries.get(property) {
            edits.push((*start, *end, replacement));
        } else {
            missing.push((property.as_str(), replacement));
        }
    }
    edits.sort_by(|left, right| right.0.cmp(&left.0));
    let mut updated = snippet.to_string();
    for (start, end, replacement) in edits {
        updated.replace_range(start..end, &replacement);
    }
    let insertion = declaration_insertion(body, &missing, kind);
    if !insertion.is_empty() {
        let adjusted_close = close + updated.len() - snippet.len();
        updated.insert_str(adjusted_close, &insertion);
    }
    Ok(updated)
}

fn declaration_insertion(body: &str, missing: &[(&str, String)], kind: DeclarationKind) -> String {
    if missing.is_empty() {
        return String::new();
    }
    let multiline = body.contains('\n');
    let mut result = String::new();
    for (property, value) in missing {
        let key = match kind {
            DeclarationKind::Css => (*property).to_string(),
            DeclarationKind::StyleObject if valid_js_identifier(property) => {
                (*property).to_string()
            }
            DeclarationKind::StyleObject => serde_json::to_string(property).unwrap_or_default(),
        };
        let separator = match kind {
            DeclarationKind::Css => ';',
            DeclarationKind::StyleObject => ',',
        };
        if multiline {
            result.push_str(&format!("\n  {key}: {value}{separator}"));
        } else {
            result.push_str(&format!(" {key}: {value}{separator}"));
        }
    }
    result
}

fn outer_braces(snippet: &str) -> Result<(usize, usize), PwaCommitError> {
    let open = snippet
        .find('{')
        .ok_or_else(|| conflict("binding range 不包含对象起始锚点"))?;
    let close = snippet
        .rfind('}')
        .ok_or_else(|| conflict("binding range 不包含对象结束锚点"))?;
    if open >= close
        || !snippet[close + 1..]
            .trim()
            .trim_end_matches(';')
            .trim()
            .is_empty()
    {
        return Err(conflict("binding range 不是完整且唯一的样式对象"));
    }
    Ok((open, close))
}

fn split_top_level(input: &str, separator: u8) -> Result<Vec<(usize, usize)>, PwaCommitError> {
    let bytes = input.as_bytes();
    let mut result = Vec::new();
    let mut start = 0;
    let mut state = ScanState::default();
    let mut index = 0;
    while index < bytes.len() {
        if state.consume(bytes, &mut index)? {
            continue;
        }
        if state.depth() == 0 && bytes[index] == separator {
            result.push((start, index));
            start = index + 1;
        }
        state.adjust(bytes[index])?;
        index += 1;
    }
    state.finish()?;
    result.push((start, input.len()));
    Ok(result)
}

fn find_top_level(input: &str, target: u8) -> Result<Option<usize>, PwaCommitError> {
    let bytes = input.as_bytes();
    let mut state = ScanState::default();
    let mut index = 0;
    while index < bytes.len() {
        if state.consume(bytes, &mut index)? {
            continue;
        }
        if state.depth() == 0 && bytes[index] == target {
            return Ok(Some(index));
        }
        state.adjust(bytes[index])?;
        index += 1;
    }
    state.finish()?;
    Ok(None)
}

#[derive(Default)]
struct ScanState {
    quote: Option<u8>,
    escaped: bool,
    block_comment: bool,
    line_comment: bool,
    parentheses: i32,
    brackets: i32,
    braces: i32,
}

impl ScanState {
    fn depth(&self) -> i32 {
        self.parentheses + self.brackets + self.braces
    }

    fn consume(&mut self, bytes: &[u8], index: &mut usize) -> Result<bool, PwaCommitError> {
        let byte = bytes[*index];
        if self.line_comment {
            self.line_comment = byte != b'\n';
            *index += 1;
            return Ok(true);
        }
        if self.block_comment {
            if byte == b'*' && bytes.get(*index + 1) == Some(&b'/') {
                self.block_comment = false;
                *index += 2;
            } else {
                *index += 1;
            }
            return Ok(true);
        }
        if let Some(quote) = self.quote {
            if self.escaped {
                self.escaped = false;
            } else if byte == b'\\' {
                self.escaped = true;
            } else if byte == quote {
                self.quote = None;
            }
            *index += 1;
            return Ok(true);
        }
        if matches!(byte, b'\'' | b'"' | b'`') {
            self.quote = Some(byte);
            *index += 1;
            return Ok(true);
        }
        if byte == b'/' && bytes.get(*index + 1) == Some(&b'*') {
            self.block_comment = true;
            *index += 2;
            return Ok(true);
        }
        if byte == b'/' && bytes.get(*index + 1) == Some(&b'/') {
            self.line_comment = true;
            *index += 2;
            return Ok(true);
        }
        Ok(false)
    }

    fn adjust(&mut self, byte: u8) -> Result<(), PwaCommitError> {
        match byte {
            b'(' => self.parentheses += 1,
            b')' => self.parentheses -= 1,
            b'[' => self.brackets += 1,
            b']' => self.brackets -= 1,
            b'{' => self.braces += 1,
            b'}' => self.braces -= 1,
            _ => {}
        }
        if self.parentheses < 0 || self.brackets < 0 || self.braces < 0 {
            return Err(conflict("binding range 内的语法括号不平衡"));
        }
        Ok(())
    }

    fn finish(&self) -> Result<(), PwaCommitError> {
        if self.quote.is_some()
            || self.block_comment
            || self.parentheses != 0
            || self.brackets != 0
            || self.braces != 0
        {
            return Err(conflict("binding range 内存在未闭合的字符串、注释或括号"));
        }
        Ok(())
    }
}

fn declaration_key(raw: &str, kind: DeclarationKind) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.starts_with("...") {
        return None;
    }
    if matches!(kind, DeclarationKind::Css) {
        return valid_source_property(raw).then(|| raw.to_string());
    }
    if (raw.starts_with('"') && raw.ends_with('"'))
        || (raw.starts_with('\'') && raw.ends_with('\''))
    {
        return Some(raw[1..raw.len() - 1].to_string());
    }
    valid_source_property(raw).then(|| raw.to_string())
}

fn validate_css_value(property: &str, value: &str) -> Result<(), PwaCommitError> {
    if value.contains(['\r', '\n', '{', '}']) {
        return Err(invalid(format!(
            "CSS 属性 {property} 的值包含不安全换行或花括号"
        )));
    }
    if find_top_level(value, b';')?.is_some() {
        return Err(invalid(format!("CSS 属性 {property} 的值包含顶层分号")));
    }
    Ok(())
}

pub(super) fn valid_source_property(value: &str) -> bool {
    if value.is_empty() || value.len() > 160 {
        return false;
    }
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || matches!(first, b'_' | b'$'))
        && bytes
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$' | b'.' | b'-'))
}

fn valid_js_identifier(value: &str) -> bool {
    !value.contains(['.', '-']) && valid_source_property(value)
}

fn trimmed_bounds(value: &str, start: usize, end: usize) -> (usize, usize) {
    let leading = value[start..end].len() - value[start..end].trim_start().len();
    let trailing = value[start..end].trim_end().len();
    (start + leading, start + trailing)
}

fn utf8_tail(value: &str, max_bytes: usize) -> &str {
    let mut start = value.len().saturating_sub(max_bytes);
    while start < value.len() && !value.is_char_boundary(start) {
        start += 1;
    }
    &value[start..]
}

fn anchored_token(haystack: &str, target: &str) -> bool {
    haystack.match_indices(target).any(|(start, _)| {
        let before = haystack[..start].chars().next_back();
        let after = haystack[start + target.len()..].chars().next();
        !before.is_some_and(identifier_character) && !after.is_some_and(identifier_character)
    })
}

fn style_anchor_matches(header: &str, preceding: &str, target: &str) -> bool {
    let anchor = if header.is_empty() {
        let start = preceding
            .rfind([';', '{', '}', '\n', '\r'])
            .map_or(0, |index| index + 1);
        preceding[start..].trim()
    } else {
        header
    };
    let Some(delimiter) = anchor.rfind(['=', ':']) else {
        return false;
    };
    anchored_token(&anchor[..delimiter], target)
}

fn identifier_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '$' | '.' | '-')
}
