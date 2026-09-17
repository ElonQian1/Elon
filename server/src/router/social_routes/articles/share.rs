//! Author-controlled public article page: static HTML with Open Graph tags, no scripts, no auth.
use super::result;
use crate::{
    project_auth::{auth_from_headers, json_error},
    store::articles::{sharing::valid_token, ArticleBlock, ArticleFault, ArticleView},
    types::AppState,
};
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/articles/:id/share",
            get(current).post(create).delete(revoke),
        )
        .route("/a/:token", get(page))
        .route("/a/:token/media/:media", get(media))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Create {
    version: i64,
}

/// Absolute origin as the visitor reaches us; the configured public URL is the fallback.
fn origin(state: &AppState, headers: &HeaderMap) -> String {
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .filter(|h| {
            !h.is_empty()
                && h.len() <= 255
                && h.bytes().all(|b| {
                    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':' | b'[' | b']')
                })
        });
    let Some(host) = host else {
        return state.public_url.trim_end_matches('/').to_string();
    };
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .filter(|p| *p == "http" || *p == "https")
        .map(str::to_string)
        .unwrap_or_else(|| {
            if state.public_url.starts_with("https://") || host.ends_with(":8443") {
                "https".into()
            } else {
                "http".into()
            }
        });
    format!("{proto}://{host}")
}

fn with_url(
    state: &AppState,
    headers: &HeaderMap,
    share: crate::store::articles::ArticleShare,
) -> serde_json::Value {
    serde_json::json!({
        "token": share.token,
        "revision": share.revision,
        "path": share.path,
        "url": format!("{}{}", origin(state, headers), share.path),
    })
}

async fn current(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let u = user!(&state, &headers);
    result(
        state.store.article_share(&u.id, &id).map(
            |share| serde_json::json!({ "share": share.map(|s| with_url(&state, &headers, s)) }),
        ),
    )
}
async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Create>,
) -> Response {
    let u = user!(&state, &headers);
    result(
        state
            .store
            .create_article_share(&u.id, &id, body.version)
            .map(|share| with_url(&state, &headers, share)),
    )
}
async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let u = user!(&state, &headers);
    result(
        state
            .store
            .revoke_article_share(&u.id, &id)
            .map(|()| serde_json::json!({"ok":true})),
    )
}

fn not_found(error: anyhow::Error) -> Response {
    let message = error
        .downcast_ref::<ArticleFault>()
        .map(|f| f.1.clone())
        .unwrap_or_else(|| "文章暂时无法打开".into());
    (
        StatusCode::NOT_FOUND,
        [("content-type", "text/html; charset=utf-8"), ("cache-control", "no-store")],
        format!(
            "<!doctype html><meta charset=utf-8><meta name=viewport content=\"width=device-width,initial-scale=1\"><title>{0}</title><p style=\"font-family:system-ui;padding:32px\">{0}</p>",
            esc(&message)
        ),
    )
        .into_response()
}

async fn page(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Response {
    if !valid_token(&token) {
        return json_error(StatusCode::NOT_FOUND, "分享链接无效");
    }
    match state.store.public_article(&token) {
        Ok(view) => {
            let base = format!("{}/a/{token}", origin(&state, &headers));
            (
                [
                    ("content-type", "text/html; charset=utf-8"),
                    ("cache-control", "public, max-age=300"),
                    ("x-content-type-options", "nosniff"),
                    ("referrer-policy", "no-referrer"),
                    (
                        "content-security-policy",
                        "default-src 'none'; img-src 'self'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'",
                    ),
                ],
                render(&view, &base),
            )
                .into_response()
        }
        Err(error) => not_found(error),
    }
}

async fn media(
    State(state): State<Arc<AppState>>,
    Path((token, media)): Path<(String, String)>,
) -> Response {
    if !valid_token(&token)
        || media.len() > 64
        || !media
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    match state.store.public_article_media(&token, &media) {
        Ok((mime, bytes)) if mime.starts_with("image/") => (
            [
                (header::CONTENT_TYPE, mime),
                (
                    header::CACHE_CONTROL,
                    "public, max-age=86400, immutable".into(),
                ),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
            ],
            bytes,
        )
            .into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

pub(super) fn esc(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c if c.is_control() && c != '\n' => {}
            c => out.push(c),
        }
    }
    out
}

fn summary(view: &ArticleView) -> String {
    let text = if view.document.summary.trim().is_empty() {
        view.document
            .blocks
            .iter()
            .find_map(|b| match b {
                ArticleBlock::Paragraph { text } if !text.trim().is_empty() => Some(text.as_str()),
                _ => None,
            })
            .unwrap_or("")
    } else {
        &view.document.summary
    };
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(200)
        .collect()
}

/// Server-rendered page; every text node is escaped and media is served from `/a/<token>/media/`.
pub(super) fn render(view: &ArticleView, base: &str) -> String {
    let doc = &view.document;
    let title = if doc.title.trim().is_empty() {
        "未命名文章"
    } else {
        doc.title.as_str()
    };
    let summary = summary(view);
    let media = |id: &str| format!("{base}/media/{}", esc(id));
    let mut html = String::with_capacity(4096);
    html.push_str("<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    html.push_str(&format!("<title>{}</title>", esc(title)));
    html.push_str("<meta name=\"robots\" content=\"noindex\"><meta property=\"og:type\" content=\"article\"><meta name=\"twitter:card\" content=\"summary_large_image\">");
    html.push_str(&format!(
        "<meta property=\"og:title\" content=\"{0}\"><meta name=\"twitter:title\" content=\"{0}\">",
        esc(title)
    ));
    if !summary.is_empty() {
        html.push_str(&format!("<meta property=\"og:description\" content=\"{0}\"><meta name=\"description\" content=\"{0}\">", esc(&summary)));
    }
    html.push_str(&format!("<meta property=\"og:url\" content=\"{}\"><meta property=\"og:site_name\" content=\"一龙文章\">", esc(base)));
    html.push_str(&format!(
        "<meta name=\"author\" content=\"{}\">",
        esc(&view.card.author_name)
    ));
    if let Some(cover) = doc.cover.as_deref().filter(|c| view.media.contains_key(*c)) {
        html.push_str(&format!("<meta property=\"og:image\" content=\"{0}\"><meta name=\"twitter:image\" content=\"{0}\">", media(cover)));
    }
    html.push_str("<style>body{margin:0;background:#15171b;color:#e9ecf1;font:16px/1.75 -apple-system,system-ui,'PingFang SC','Microsoft YaHei',sans-serif}article{max-width:720px;margin:0 auto;padding:24px 20px 56px}h1{font-size:26px;line-height:1.35;margin:8px 0 6px}h2{font-size:20px;margin:28px 0 8px}small{color:#9aa3b2}p{margin:14px 0;overflow-wrap:anywhere;white-space:pre-wrap}blockquote{margin:16px 0;padding:8px 16px;border-left:3px solid #4f8cff;color:#c5d0df;background:#1c2027;border-radius:6px}img{max-width:100%;border-radius:10px;display:block}figure{margin:18px 0}figcaption{color:#9aa3b2;font-size:13px;margin-top:6px}.abstract{color:#c5d0df;background:#1c2027;padding:12px 14px;border-radius:8px}footer{margin-top:40px;color:#7f8896;font-size:13px;border-top:1px solid #262b33;padding-top:14px}</style></head><body><article>");
    if let Some(cover) = doc.cover.as_deref().filter(|c| view.media.contains_key(*c)) {
        html.push_str(&format!("<img src=\"{}\" alt=\"文章封面\">", media(cover)));
    }
    html.push_str(&format!(
        "<h1>{}</h1><small>{} · {}</small>",
        esc(title),
        esc(&view.card.author_name),
        esc(view.card.updated_at.get(..10).unwrap_or(""))
    ));
    if !doc.summary.trim().is_empty() {
        html.push_str(&format!("<p class=\"abstract\">{}</p>", esc(&doc.summary)));
    }
    for block in &doc.blocks {
        match block {
            ArticleBlock::Paragraph { text } => html.push_str(&format!("<p>{}</p>", esc(text))),
            ArticleBlock::Heading { text } => html.push_str(&format!("<h2>{}</h2>", esc(text))),
            ArticleBlock::Quote { text } => {
                html.push_str(&format!("<blockquote>{}</blockquote>", esc(text)))
            }
            ArticleBlock::Image { media_id, caption } => {
                if view.media.contains_key(media_id) {
                    html.push_str(&format!(
                        "<figure><img src=\"{}\" alt=\"{}\" loading=\"lazy\"><figcaption>{}</figcaption></figure>",
                        media(media_id),
                        esc(caption),
                        esc(caption)
                    ));
                }
            }
        }
    }
    html.push_str("<footer>由一龙发布 · 作者可随时关闭此公开链接</footer></article></body></html>");
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::articles::{ArticleCard, ArticleDocument};

    #[test]
    fn public_page_escapes_text_and_only_links_known_media() {
        let view = ArticleView {
            card: ArticleCard {
                id: "article_1".into(),
                revision: 1,
                title: String::new(),
                summary: String::new(),
                author_id: "u".into(),
                author_name: "<b>作者</b>".into(),
                cover_data_url: None,
                status: "published".into(),
                updated_at: "2026-09-18T01:02:03Z".into(),
            },
            document: ArticleDocument {
                title: "标题 <script>alert(1)</script>".into(),
                summary: String::new(),
                cover: Some("m_cover".into()),
                blocks: vec![
                    ArticleBlock::Paragraph {
                        text: "第一段 \"引号\" & 符号".into(),
                    },
                    ArticleBlock::Image {
                        media_id: "m_missing".into(),
                        caption: "x".into(),
                    },
                ],
            },
            media: [("m_cover".to_string(), String::new())]
                .into_iter()
                .collect(),
        };
        let html = render(&view, "https://example.test/a/s_abc");
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!html.contains("<script>"));
        assert!(html.contains("og:image\" content=\"https://example.test/a/s_abc/media/m_cover\""));
        assert!(html.contains("og:description\" content=\"第一段 &quot;引号&quot; &amp; 符号\""));
        assert!(!html.contains("m_missing"));
        assert!(html.contains("&lt;b&gt;作者&lt;/b&gt; · 2026-09-18"));
        assert!(valid_token("s_0123abcd") && !valid_token("s_../x") && !valid_token("article_1"));
    }
}
