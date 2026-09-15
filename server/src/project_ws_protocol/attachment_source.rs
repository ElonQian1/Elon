//! Small client-derived metadata. Never fetch or trust a QR destination server-side.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "SourceInput")]
pub struct AttachmentSource {
    pub version: u8,
    pub url: String,
    pub method: String,
}

#[derive(Deserialize)]
struct SourceInput {
    version: u8,
    url: String,
    method: String,
}

impl TryFrom<SourceInput> for AttachmentSource {
    type Error = &'static str;
    fn try_from(input: SourceInput) -> Result<Self, Self::Error> {
        if input.version != 1 || !matches!(input.method.as_str(), "qr" | "share") {
            return Err("unsupported image source metadata");
        }
        let raw = &input.url;
        if !raw.to_ascii_lowercase().starts_with("https://")
            && !raw.to_ascii_lowercase().starts_with("http://")
        {
            return Err("image source requires an absolute HTTP(S) URL");
        }
        if raw.len() > 4096
            || raw
                .chars()
                .any(|c| c.is_control() || c.is_whitespace() || c == '\\')
        {
            return Err("invalid image source URL");
        }
        let parsed = reqwest::Url::parse(raw).map_err(|_| "invalid image source URL")?;
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            return Err("image source requires an HTTP(S) URL without credentials");
        }
        Ok(Self {
            version: 1,
            url: input.url,
            method: input.method,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn read(url: &str) -> Result<AttachmentSource, serde_json::Error> {
        serde_json::from_value(serde_json::json!({"version":1,"url":url,"method":"qr"}))
    }
    #[test]
    fn preserves_query_and_fragment_without_fetching() {
        let url = "https://mp.weixin.qq.com/s/test?start=1&end=2&scene=90#part";
        assert_eq!(read(url).unwrap().url, url);
    }
    #[test]
    fn rejects_unsafe_and_oversized_sources() {
        for url in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "https://user:pass@example.com",
            "/article",
            "https://a.test/\nfoo",
            "https://a.test\\@b.test/",
        ] {
            assert!(read(url).is_err(), "{url}");
        }
        assert!(read(&format!("https://a.test/{}", "x".repeat(4096))).is_err());
        assert!(serde_json::from_value::<AttachmentSource>(
            serde_json::json!({"version":2,"url":"https://a.test","method":"qr"})
        )
        .is_err());
    }
}
