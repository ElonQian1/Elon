use super::policy;
use reqwest::Url;

#[derive(Default, Debug)]
pub(super) struct Metadata {
    pub title: String,
    pub description: String,
    pub author: String,
    pub image: Option<String>,
    pub image_needs_check: bool,
}

pub(super) const DESCRIPTION_MAX: usize = 300;

pub(super) fn clean(value: &str, max: usize) -> String {
    let decoded = decode(value);
    decoded
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|c| !c.is_control())
        .take(max)
        .collect()
}

fn decode(value: &str) -> String {
    // Decode individually so one unknown HTML entity cannot suppress all known ones.
    let mut out = String::new();
    let mut rest = value;
    while let Some(pos) = rest.find('&') {
        out.push_str(&rest[..pos]);
        rest = &rest[pos..];
        if let Some(end) = rest.find(';').filter(|n| *n < 16) {
            let entity = &rest[..=end];
            if entity == "&nbsp;" {
                out.push(' ');
            } else {
                out.push_str(
                    &quick_xml::escape::unescape(entity).unwrap_or_else(|_| entity.into()),
                );
            }
            rest = &rest[end + 1..];
        } else {
            out.push('&');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
}

// Bounded HTML head tokenizer; script/style/comment bodies are never metadata.
pub(super) fn parse(html: &str, base: &Url) -> Metadata {
    let mut out = Metadata::default();
    let mut fallback = String::new();
    let lower = html.to_ascii_lowercase();
    let mut cursor = 0;
    while let Some(offset) = lower[cursor..].find('<') {
        let start = cursor + offset;
        if lower[start..].starts_with("<!--") {
            cursor = lower[start..]
                .find("-->")
                .map(|n| start + n + 3)
                .unwrap_or(html.len());
            continue;
        }
        let Some(end) = tag_end(html, start) else {
            break;
        };
        cursor = end + 1;
        let tag = &html[start + 1..end];
        let name = tag
            .split_ascii_whitespace()
            .next()
            .unwrap_or("")
            .trim_end_matches('/')
            .to_ascii_lowercase();
        if name == "/head" || name == "body" {
            break;
        }
        if name == "script" || name == "style" {
            cursor = lower[cursor..]
                .find(&format!("</{name}"))
                .map(|n| cursor + n)
                .unwrap_or(html.len());
        } else if name == "title" {
            if let Some(n) = lower[cursor..].find("</title") {
                fallback = clean(&html[cursor..cursor + n], 160);
            }
        } else if name == "meta" {
            let attrs = attributes(&tag[4..]);
            let key = attrs
                .iter()
                .find(|(k, _)| k == "property" || k == "name")
                .map(|(_, v)| v.to_ascii_lowercase())
                .unwrap_or_default();
            let value = attrs
                .iter()
                .find(|(k, _)| k == "content")
                .map(|(_, v)| v.as_str())
                .unwrap_or("");
            match key.as_str() {
                "og:title" => out.title = clean(value, 160),
                "twitter:title" if out.title.is_empty() => out.title = clean(value, 160),
                "og:description" => out.description = clean(value, DESCRIPTION_MAX),
                "twitter:description" | "description" if out.description.is_empty() => {
                    out.description = clean(value, DESCRIPTION_MAX)
                }
                "author" | "og:article:author" => out.author = clean(value, 80),
                "og:image" | "twitter:image" | "og:image:secure_url" => {
                    let decoded = decode(value);
                    let mut candidate = base.join(&decoded).ok();
                    let upgraded = candidate.as_ref().is_some_and(|u| u.scheme() == "http");
                    if upgraded {
                        if let Some(url) = &mut candidate {
                            let _ = url.set_scheme("https");
                        }
                    }
                    if let Some(image) = candidate.and_then(|u| policy::image_url(u.as_str(), base))
                    {
                        if image.contains("/fe-platform/")
                            || image.to_ascii_lowercase().contains("logo")
                        {
                            continue;
                        }
                        if out.image.is_none()
                            || out
                                .image
                                .as_ref()
                                .is_some_and(|v| v.contains("fe-platform/") || v.contains("logo"))
                        {
                            out.image = Some(image);
                            out.image_needs_check = upgraded;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if out.title.is_empty() {
        out.title = fallback;
    }
    out
}

fn tag_end(html: &str, start: usize) -> Option<usize> {
    let mut quote = 0;
    for (offset, b) in html.as_bytes()[start + 1..].iter().copied().enumerate() {
        if quote != 0 {
            if b == quote {
                quote = 0;
            }
        } else if b == b'\'' || b == b'"' {
            quote = b;
        } else if b == b'>' {
            return Some(start + 1 + offset);
        }
    }
    None
}

fn attributes(tag: &str) -> Vec<(String, String)> {
    let b = tag.as_bytes();
    let mut i = 0;
    let mut attrs = Vec::new();
    while i < b.len() && attrs.len() < 32 {
        while i < b.len() && (b[i].is_ascii_whitespace() || b[i] == b'/') {
            i += 1;
        }
        let start = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'=' {
            i += 1;
        }
        let key = tag[start..i].to_ascii_lowercase();
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i == b.len() || b[i] != b'=' {
            continue;
        }
        i += 1;
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i == b.len() {
            break;
        }
        let quote = if b[i] == b'\'' || b[i] == b'"' {
            let q = b[i];
            i += 1;
            q
        } else {
            0
        };
        let start = i;
        while i < b.len()
            && if quote == 0 {
                !b[i].is_ascii_whitespace()
            } else {
                b[i] != quote
            }
        {
            i += 1;
        }
        attrs.push((key, tag[start..i].to_string()));
        if quote != 0 && i < b.len() {
            i += 1;
        }
    }
    attrs
}

pub(super) fn plain_snippet(html: &str) -> String {
    let mut text = String::new();
    let mut inside = false;
    for c in html.chars() {
        match c {
            '<' => {
                inside = true;
                text.push(' ');
            }
            '>' => inside = false,
            _ if !inside => text.push(c),
            _ => {}
        }
    }
    clean(&text, 160)
}
