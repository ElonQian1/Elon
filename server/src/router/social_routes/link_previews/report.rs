//! Member read-back: metadata a signed-in client observed on the original page it opened.
//! Everything is re-validated here; only same-identity, allow-listed fields ever reach others.
use super::{metadata, policy};
use reqwest::Url;
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Read {
    pub(super) schema: u8,
    pub(super) original: String,
    pub(super) url: String,
    pub(super) article: bool,
    pub(super) title: String,
    #[serde(default)]
    pub(super) author: String,
    #[serde(default)]
    pub(super) description: String,
    #[serde(default)]
    pub(super) image: Option<String>,
}

pub(super) struct Accepted {
    pub title: String,
    pub author: String,
    pub description: String,
    pub image: Option<String>,
}

fn digits(value: &str) -> bool {
    (5..=24).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_digit())
}

fn handle(value: &str) -> bool {
    (1..=15).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn locale(value: &str) -> bool {
    let (lang, region) = value.split_once('-').unwrap_or((value, ""));
    lang.len() == 2
        && lang.bytes().all(|b| b.is_ascii_lowercase())
        && (region.is_empty()
            || (region.len() == 2 && region.bytes().all(|b| b.is_ascii_uppercase())))
}

// Mirrors `identity()` in social_link_read_adapter.js; both sides must agree on the same ID.
pub(super) fn identity(url: &Url) -> Option<String> {
    let path = url.path().trim_end_matches('/');
    let parts: Vec<&str> = path.split('/').skip(1).collect();
    match url.host_str()? {
        "mp.weixin.qq.com" => match parts.as_slice() {
            ["s"] => Some(format!(
                "wechat:{path}{}",
                url.query().map(|q| format!("?{q}")).unwrap_or_default()
            )),
            ["s", slug] if !slug.is_empty() => Some(format!("wechat:{path}")),
            _ => None,
        },
        "binance.com" | "www.binance.com" | "app.binance.com" => match parts.as_slice() {
            ["square", "post" | "article", id] if digits(id) => Some(format!("binance:{id}")),
            [lang, "square", "post" | "article", id] if locale(lang) && digits(id) => {
                Some(format!("binance:{id}"))
            }
            ["uni-qr", "cpos", id] if digits(id) => Some(format!("binance:{id}")),
            _ => None,
        },
        "x.com" | "www.x.com" | "twitter.com" | "www.twitter.com" | "mobile.twitter.com" => {
            match parts.as_slice() {
                [user, "status", id] if handle(user) && digits(id) => Some(format!("x-post:{id}")),
                ["i", "web", "status", id] if digits(id) => Some(format!("x-post:{id}")),
                ["i", "article", id] if digits(id) => Some(format!("x-article:{id}")),
                _ => None,
            }
        }
        _ => None,
    }
}

fn image(value: Option<&str>, kind: &str, base: &Url) -> Option<String> {
    let raw = value?;
    let url = policy::public_url(raw)?;
    let host = url.host_str()?;
    let suffix = |domain: &str| host == domain || host.ends_with(&format!(".{domain}"));
    let path = url.path().to_ascii_lowercase();
    let ok = match kind {
        "wechat" => suffix("qpic.cn"),
        "binance" => {
            suffix("bnbstatic.com") && !["logo", "avatar", "icon"].iter().any(|w| path.contains(w))
        }
        "x-post" | "x-article" => {
            host == "pbs.twimg.com"
                && [
                    "/media/",
                    "/card_img/",
                    "/amplify_video_thumb/",
                    "/ext_tw_video_thumb/",
                    "/tweet_video_thumb/",
                ]
                .iter()
                .any(|p| path.starts_with(p))
        }
        _ => false,
    };
    ok.then(|| policy::image_url(url.as_str(), base)).flatten()
}

fn generic(title: &str) -> bool {
    let lower = title.to_ascii_lowercase();
    let exact = [
        "x",
        "twitter",
        "binance",
        "binance square",
        "币安",
        "币安广场",
        "微信公众号",
        "微信公众号文章",
        "微信公众平台",
        "环境异常",
        "安全验证",
        "访问验证",
        "access denied",
        "log in to x",
        "sign in to x",
        "登录",
        "登入",
    ];
    let prefixes = [
        "just a moment",
        "binance -",
        "binance square -",
        "log in",
        "sign in",
        "page not found",
        "this post ",
        "this page ",
        "something went wrong",
        "内容已删除",
        "该内容已",
    ];
    exact.contains(&lower.as_str()) || prefixes.iter().any(|p| lower.starts_with(p))
}

pub(super) fn validate(original: &Url, read: &Read) -> Result<Accepted, &'static str> {
    let same_original =
        policy::public_url(&read.original).is_some_and(|u| u.as_str() == original.as_str());
    if read.schema != 1 || !read.article || !same_original {
        return Err("回填内容与链接不一致");
    }
    let expected = identity(original).ok_or("该链接不支持成员回填")?;
    let seen = policy::public_url(&read.url).and_then(|u| identity(&u));
    if seen.as_deref() != Some(expected.as_str()) {
        return Err("回填页面与链接不一致");
    }
    let title = metadata::clean(&read.title, 160);
    if title.is_empty() || generic(&title) {
        return Err("回填标题无效");
    }
    let kind = expected.split(':').next().unwrap_or("");
    Ok(Accepted {
        title,
        author: metadata::clean(&read.author, 80),
        description: metadata::clean(&read.description, metadata::DESCRIPTION_MAX),
        image: image(read.image.as_deref(), kind, original),
    })
}

const LIMIT: u32 = 40;
const WINDOW: Duration = Duration::from_secs(3600);
static RATE: LazyLock<Mutex<HashMap<String, (Instant, u32)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(super) fn allow(user: &str) -> bool {
    let mut rate = RATE.lock().unwrap_or_else(|e| e.into_inner());
    rate.retain(|_, (at, _)| at.elapsed() < WINDOW);
    let entry = rate.entry(user.to_string()).or_insert((Instant::now(), 0));
    entry.1 += 1;
    entry.1 <= LIMIT
}
