use super::{fail, Result};

pub(crate) fn validate_text(value: &str) -> Result<()> {
    if value
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err(fail(400, "Invalid snapshot text control character"));
    }
    let words: Vec<_> = value
        .split(|c: char| c.is_whitespace() || "<>()[]{}\"'`".contains(c))
        .filter(|s| !s.is_empty())
        .collect();
    for (index, word) in words.iter().enumerate() {
        let lower = word.to_ascii_lowercase();
        let jwt = word.split('.').collect::<Vec<_>>();
        if (word.starts_with("sk-") && word.len() >= 32 && secret_like(word))
            || (word.starts_with("eyJ") && jwt.len() == 3 && jwt.iter().all(|v| v.len() >= 12))
            || (lower == "bearer" && words.get(index + 1).is_some_and(|v| secret_like(v)))
        {
            return Err(fail(400, "Credential value is not snapshot content"));
        }
        let name = lower.trim_end_matches([':', '=']);
        if sensitive_name(name) {
            let next = words
                .get(index + 1)
                .copied()
                .unwrap_or("")
                .trim_start_matches([':', '=']);
            let next = if next.is_empty() {
                words.get(index + 2).copied().unwrap_or("")
            } else {
                next
            };
            if secret_like(next) {
                return Err(fail(400, "Credential value is not snapshot content"));
            }
        }
        if let Some((name, secret)) = word.split_once('=') {
            if sensitive_name(name) && secret_like(secret) {
                return Err(fail(400, "Credential value is not snapshot content"));
            }
        }
        // Scheme examples are inert prose, not resource references. Media uses asset_id;
        // readers separately allowlist link actions and never fetch Markdown images.
        if lower.starts_with("http://")
            || lower.starts_with("https://")
            || lower
                .strip_prefix("//")
                .is_some_and(|v| v.split('/').next().unwrap_or("").contains('.'))
        {
            validate_url(word)?;
        }
    }
    Ok(())
}

fn secret_like(value: &str) -> bool {
    let value = value.trim_matches(|c: char| !c.is_ascii_alphanumeric() && !"_-+/=.".contains(c));
    if value.len() < 20
        || value
            .bytes()
            .any(|c| !c.is_ascii_alphanumeric() && !b"_-+/=.".contains(&c))
    {
        return false;
    }
    let distinct = value
        .bytes()
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    distinct >= 12
        && value.bytes().any(|c| c.is_ascii_digit())
        && value.bytes().any(|c| c.is_ascii_alphabetic())
}

fn sensitive_name(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "api_key"
            | "apikey"
            | "access_token"
            | "accesstoken"
            | "refresh_token"
            | "refreshtoken"
            | "session_token"
            | "sessiontoken"
            | "session"
            | "sessionid"
            | "token"
            | "password"
            | "authorization"
            | "__secure-next-auth.session-token"
    )
}

fn validate_url(word: &str) -> Result<()> {
    let url_text = if word.starts_with("//") {
        format!("https:{word}")
    } else {
        word.to_owned()
    };
    let url = reqwest::Url::parse(&url_text).map_err(|_| fail(400, "Invalid citation URL"))?;
    let host = url.host_str().unwrap_or_default();
    let numeric = host
        .trim_matches(['[', ']'])
        .parse::<std::net::IpAddr>()
        .is_ok();
    let provider_private = host == "chatgpt.com"
        && ["/c/", "/share/", "/files/", "/backend-api/"]
            .iter()
            .any(|p| url.path().starts_with(p))
        || host == "chat.openai.com"
        || ["oaiusercontent.com", "googleusercontent.com"]
            .iter()
            .any(|h| host == *h || host.ends_with(&format!(".{h}")))
        || url.path().starts_with("/backend-api/");
    let secret_query = url.query_pairs().any(|(key, _)| {
        sensitive_name(&key)
            || matches!(
                key.to_ascii_lowercase().as_str(),
                "sig"
                    | "signature"
                    | "x-amz-signature"
                    | "x-amz-credential"
                    | "x-goog-signature"
                    | "x-goog-credential"
            )
    });
    if host.is_empty()
        || host == "localhost"
        || !host.contains('.')
        || numeric
        || [".local", ".internal", ".localhost"]
            .iter()
            .any(|suffix| host.ends_with(suffix))
        || !url.username().is_empty()
        || url.password().is_some()
        || provider_private
        || secret_query
    {
        return Err(fail(
            400,
            "Private resource or credential-bearing URL is not shareable",
        ));
    }
    Ok(())
}
