//! Read public, same-item hydration data as JSON, never execute downloaded JavaScript.
use super::{metadata, policy};
use reqwest::Url;
use serde_json::Value;

pub(super) fn identity(url: &Url) -> Option<(&'static str, &str)> {
    let p = url.path().trim_end_matches('/');
    let id = p.rsplit('/').next()?;
    match url.host_str()? {
        "www.douyin.com" | "douyin.com" | "www.iesdouyin.com"
            if (p == format!("/video/{id}") || p == format!("/share/video/{id}"))
                && (5..=24).contains(&id.len())
                && id.bytes().all(|b| b.is_ascii_digit()) =>
        {
            Some(("douyin", id))
        }
        "www.xiaohongshu.com" | "xiaohongshu.com"
            if (p == format!("/explore/{id}") || p == format!("/discovery/item/{id}"))
                && id.len() == 24
                && id
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) =>
        {
            Some(("xiaohongshu", id))
        }
        _ => None,
    }
}

pub(super) fn parse(html: &str, url: &Url) -> Option<metadata::Metadata> {
    let (kind, id) = identity(url)?;
    if html.len() > 2 * 1024 * 1024 {
        return None;
    }
    let lower = html.to_ascii_lowercase();
    let mut cursor = 0;
    while let Some(at) = lower[cursor..].find("<script") {
        let start = cursor + at;
        let end = metadata::tag_end(html, start)?;
        let close = end + 1 + lower[end + 1..].find("</script>")?;
        let attrs = metadata::attributes(&html[start + 7..end]);
        let text = html[end + 1..close].trim();
        cursor = close + 9;
        let json = if kind == "douyin" && attrs.iter().any(|(k, v)| k == "id" && v == "RENDER_DATA")
        {
            let decoder = Url::parse(&format!(
                "https://preview.invalid/?value={}",
                text.replace('+', "%2B")
            ))
            .ok()?;
            decoder.query_pairs().next()?.1.into_owned()
        } else if kind == "xiaohongshu" {
            let Some(value) = text
                .strip_prefix("window.__INITIAL_STATE__")
                .and_then(|s| s.trim_start().strip_prefix('='))
            else {
                continue;
            };
            json_undefined(value.trim_start())
        } else {
            continue;
        };
        let Some(Ok(value)) = serde_json::Deserializer::from_str(&json)
            .into_iter::<Value>()
            .next()
        else {
            continue;
        };
        if let Some(item) = find_item(&value, id) {
            if let Some(meta) = fields(item, kind, url) {
                return Some(meta);
            }
        }
    }
    None
}

fn find_item<'a>(root: &'a Value, id: &str) -> Option<&'a Value> {
    let mut stack = vec![(root, 0)];
    let mut visited = 0;
    while let Some((value, depth)) = stack.pop() {
        visited += 1;
        if visited > 6000 {
            break;
        }
        if ["aweme_id", "awemeId", "noteId", "note_id"]
            .iter()
            .any(|k| value.get(k).and_then(Value::as_str) == Some(id))
            && (value.get("video").is_some() || value.get("imageList").is_some())
        {
            return Some(value);
        }
        if depth >= 16 {
            continue;
        }
        match value {
            Value::Object(map) => stack.extend(map.values().take(128).map(|v| (v, depth + 1))),
            Value::Array(list) => stack.extend(list.iter().take(128).map(|v| (v, depth + 1))),
            _ => {}
        }
    }
    None
}

fn fields(item: &Value, kind: &str, base: &Url) -> Option<metadata::Metadata> {
    let text = |paths: &[&str]| {
        paths
            .iter()
            .find_map(|p| {
                item.pointer(p)
                    .and_then(Value::as_str)
                    .filter(|s| !s.trim().is_empty())
            })
            .unwrap_or("")
            .to_string()
    };
    let title = metadata::clean(&text(&["/title", "/desc", "/item_title"]), 160);
    if title.is_empty() {
        return None;
    }
    let author = metadata::clean(
        &text(&["/author/nickname", "/user/nickname", "/user/nickName"]),
        80,
    );
    let raw = if kind == "douyin" {
        text(&["/video/origin_cover/url_list/0", "/video/cover/url_list/0"])
    } else {
        text(&[
            "/imageList/0/urlDefault",
            "/imageList/0/url",
            "/imageList/0/infoList/0/url",
        ])
    };
    let upgraded = raw.starts_with("http://");
    let raw = if upgraded {
        raw.replacen("http://", "https://", 1)
    } else {
        raw
    };
    let image = policy::image_url(&raw, base)?;
    let u = Url::parse(&image).ok()?;
    let suffix = |s: &str| {
        u.host_str()
            .is_some_and(|h| h == s || h.ends_with(&format!(".{s}")))
    };
    if !(if kind == "douyin" {
        suffix("douyinpic.com") || suffix("byteimg.com")
    } else {
        suffix("xhscdn.com")
    }) || ["logo", "avatar", "icon", "fe-platform", "gaosi"]
        .iter()
        .any(|s| u.path().to_ascii_lowercase().contains(s))
    {
        return None;
    }
    Some(metadata::Metadata {
        title,
        author,
        image: Some(image),
        image_needs_check: upgraded,
        ..Default::default()
    })
}

// XHS hydration contains unquoted undefined values. Normalize only complete JSON value tokens;
// quoted text and arbitrary expressions remain untouched and must still pass the JSON parser.
fn json_undefined(raw: &str) -> String {
    let b = raw.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let (mut i, mut quoted, mut escaped) = (0, false, false);
    while i < b.len() {
        let c = b[i];
        if quoted {
            out.push(c);
            i += 1;
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                quoted = false;
            }
        } else if c == b'"' {
            quoted = true;
            out.push(c);
            i += 1;
        } else if b[i..].starts_with(b"undefined")
            && out
                .iter()
                .rev()
                .find(|v| !v.is_ascii_whitespace())
                .is_some_and(|v| matches!(*v, b':' | b'[' | b','))
            && b[i + 9..]
                .iter()
                .find(|v| !v.is_ascii_whitespace())
                .is_some_and(|v| matches!(*v, b',' | b']' | b'}'))
        {
            out.extend_from_slice(b"null");
            i += 9;
        } else {
            out.push(c);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_xhs_note_with_undefined_and_signed_cover() {
        let url = Url::parse("https://www.xiaohongshu.com/discovery/item/0123456789abcdef01234567?xsec_token=fixture").unwrap();
        let html = r#"<html><body><script>window.__INITIAL_STATE__={"note":{"unused":undefined,"noteDetailMap":{"note":{"noteId":"0123456789abcdef01234567","title":"undefined in a title","user":{"nickname":"Author"},"imageList":[{"urlDefault":"https://sns-webpic-qc.xhscdn.com/poster.jpg?sign=fixture"}]}}}};</script></body></html>"#;
        let meta = parse(html, &url).unwrap();
        assert_eq!(meta.title, "undefined in a title");
        assert!(meta.image.unwrap().ends_with("?sign=fixture"));
        assert!(parse(&html.replace("noteId", "otherId"), &url).is_none());
        assert!(parse(
            &html.replace("sns-webpic-qc.xhscdn.com", "xhscdn.com.evil.example"),
            &url
        )
        .is_none());
        assert!(parse(&html.replace("poster.jpg", "avatar.jpg"), &url).is_none());
    }
    #[test]
    fn douyin_percent_json_same_id_only() {
        let url = Url::parse("https://www.douyin.com/video/12345678").unwrap();
        let data = r#"{"aweme_id":"12345678","desc":"A + B","video":{"cover":{"url_list":["https://p3.douyinpic.com/poster.jpg"]}}}"#;
        let encoded: String = data.bytes().map(|b| format!("%{b:02X}")).collect();
        let html = format!("<script id='RENDER_DATA' type='application/json'>{encoded}</script>");
        assert_eq!(parse(&html, &url).unwrap().title, "A + B");
        assert!(parse(
            &html,
            &Url::parse("https://www.douyin.com/video/87654321").unwrap()
        )
        .is_none());
        assert!(
            identity(&Url::parse("https://www.douyin.com/video/12345/12345678").unwrap()).is_none()
        );
        assert!(parse("<script>throw new Error('not data')</script>", &url).is_none());
        assert_eq!(
            json_undefined(r#"{"a":undefined,"b":"undefined","c":[undefined]}"#),
            r#"{"a":null,"b":"undefined","c":[null]}"#
        );
    }
}
