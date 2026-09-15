use reqwest::Url;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(super) struct Embed {
    pub kind: &'static str,
    pub id: String,
    pub url: String,
}

pub(super) fn public_url(value: &str) -> Option<Url> {
    if value.len() > 4096 || value.chars().any(char::is_control) {
        return None;
    }
    let url = Url::parse(value).ok()?;
    (url.scheme() == "https"
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
        && url
            .host_str()
            .is_some_and(|h| h.contains('.') && h != "localhost"))
    .then_some(url)
}

pub(super) fn site(url: &Url) -> &'static str {
    match url.host_str().unwrap_or("") {
        "mp.weixin.qq.com" => "微信公众号",
        "douyin.com" | "www.douyin.com" | "v.douyin.com" | "www.iesdouyin.com" => "抖音",
        "xiaohongshu.com" | "www.xiaohongshu.com" | "xhslink.com" | "www.xhslink.com" => "小红书",
        "bilibili.com" | "www.bilibili.com" | "m.bilibili.com" | "b23.tv" => "哔哩哔哩",
        "binance.com" | "www.binance.com" => "币安广场",
        "x.com" | "www.x.com" | "twitter.com" | "www.twitter.com" | "mobile.twitter.com"
        | "t.co" => "X",
        _ => "网页",
    }
}

// Fetch only the six providers in this release; arbitrary links still open normally.
pub(super) fn fetchable(url: &Url) -> bool {
    site(url) != "网页"
}

pub(super) fn embed(url: &Url) -> Option<Embed> {
    let parts: Vec<_> = url.path_segments()?.collect();
    match site(url) {
        "哔哩哔哩" => {
            let id = parts.windows(2).find(|p| p[0] == "video")?[1];
            if !id.starts_with("BV")
                || id.len() != 12
                || !id.bytes().all(|b| b.is_ascii_alphanumeric())
            {
                return None;
            }
            let mut player = Url::parse("https://player.bilibili.com/player.html").ok()?;
            player
                .query_pairs_mut()
                .append_pair("bvid", id)
                .append_pair("autoplay", "0")
                .append_pair("poster", "1");
            for (key, value) in url.query_pairs() {
                let limit = match key.as_ref() {
                    "t" => 604800,
                    "p" => 10000,
                    _ => continue,
                };
                if value
                    .parse::<u32>()
                    .is_ok_and(|n| n <= limit && (key != "p" || n > 0))
                {
                    player.query_pairs_mut().append_pair(&key, &value);
                }
            }
            Some(Embed {
                kind: "bilibili",
                id: id.into(),
                url: player.into(),
            })
        }
        "抖音" => {
            let id = parts.windows(2).find(|p| p[0] == "video")?[1];
            if !digits(id) {
                return None;
            }
            Some(Embed {
                kind: "douyin",
                id: id.into(),
                url: format!("https://open.douyin.com/player/video?vid={id}&autoplay=0"),
            })
        }
        "X" => {
            let id = parts.windows(2).find(|p| p[0] == "status")?[1];
            if !digits(id) {
                return None;
            }
            Some(Embed {
                kind: "x",
                id: id.into(),
                url: format!("https://x.com/i/status/{id}"),
            })
        }
        _ => None,
    }
}

fn digits(value: &str) -> bool {
    (5..=24).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_digit())
}

// Covers are direct, credential-free HTTPS loads, limited to provider image CDNs.
pub(super) fn image_url(value: &str, base: &Url) -> Option<String> {
    let url = base.join(value).ok()?;
    public_url(url.as_str())?;
    let host = url.host_str()?;
    let allowed = [
        "qpic.cn",
        "qlogo.cn",
        "hdslb.com",
        "douyinpic.com",
        "douyincdn.com",
        "byteimg.com",
        "bytecdn.cn",
        "xhscdn.com",
        "xiaohongshu.com",
        "bnbstatic.com",
        "twimg.com",
    ];
    allowed
        .iter()
        .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
        .then(|| url.to_string())
}
