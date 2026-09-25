//! Resolve only the current owner's personal ChatGPT document for read-only research.
use super::{provider, resolve_owner_fingerprint, window_label};
use tauri::{AppHandle, Manager, Url};

pub(crate) fn chatgpt_label(app: &AppHandle, owner_key: &str) -> Result<String, String> {
    let provider = provider("chatgpt")?;
    let fingerprint = resolve_owner_fingerprint(app, provider, owner_key)?;
    let label = window_label(provider, &fingerprint);
    let page = app.get_webview(&label).ok_or("chatgpt_session_not_open")?;
    if !official_document(&page.url().map_err(|_| "chatgpt_document_unavailable")?) {
        return Err("chatgpt_document_not_ready".into());
    }
    // Never create another login profile, navigate, focus, or select a group document.
    Ok(label)
}

fn official_document(url: &Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("chatgpt.com")
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_requires_the_exact_official_origin() {
        for url in ["https://chatgpt.com/", "https://chatgpt.com/c/example"] {
            assert!(official_document(&Url::parse(url).unwrap()));
        }
        for url in [
            "http://chatgpt.com/",
            "https://chatgpt.com.example/",
            "https://user@chatgpt.com/",
            "https://chatgpt.com:444/",
            "https://auth.openai.com/",
        ] {
            assert!(!official_document(&Url::parse(url).unwrap()));
        }
    }
}
