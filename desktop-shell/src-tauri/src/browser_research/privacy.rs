//! Targeted credential exclusion, not a universal secret detector or JavaScript/HTML parser.
//! JSON credential keys, quoted assignments, form/query pairs, credential meta/input values,
//! Bearer/JWT and URL userinfo are covered. Ambiguous token/sessionId scalar values are
//! excluded only when credential-shaped (>=16 URL-token characters); short tickers and
//! structured business token objects survive. Obfuscated/encrypted custom formats need an adapter.
use super::model::BODY_LIMIT;
#[path = "privacy_json.rs"]
mod json_keys;
use regex::{Captures, Regex};
use serde_json::Value;
use std::sync::OnceLock;

const EXCLUDED: &str = "[credential_excluded]";
fn normalized(key: &str) -> String {
    key.chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase()
}
fn sensitive(key: &str) -> bool {
    matches!(
        normalized(key).as_str(),
        "authorization"
            | "proxyauthorization"
            | "cookie"
            | "cookies"
            | "setcookie"
            | "password"
            | "passwd"
            | "accesstoken"
            | "refreshtoken"
            | "idtoken"
            | "csrftoken"
            | "xcsrftoken"
            | "xsrftoken"
            | "xxsrftoken"
            | "sessiontoken"
            | "sessionkey"
            | "apikey"
            | "xmbxapikey"
            | "secretkey"
            | "apisecret"
            | "clientsecret"
            | "listenkey"
            | "signature"
            | "secret"
            | "credential"
            | "credentials"
    )
}
fn credential_value(key: &str, value: &str) -> bool {
    sensitive(key)
        || (matches!(normalized(key).as_str(), "token" | "sessionid")
            && value.len() >= 16
            && value
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || b"-_.~+/=".contains(&v)))
}
fn clean_value(value: &mut Value, depth: usize) {
    if depth > 64 {
        *value = Value::String("[depth_limited]".into());
        return;
    }
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                if sensitive(key) || value.as_str().is_some_and(|v| credential_value(key, v)) {
                    *value = Value::String(EXCLUDED.into());
                } else {
                    clean_value(value, depth + 1);
                }
            }
        }
        Value::Array(items) => {
            for value in items {
                clean_value(value, depth + 1);
            }
        }
        Value::String(text) => *text = clean_text(text),
        _ => {}
    }
}

struct Patterns {
    quoted: Regex,
    pair: Regex,
    key: Regex,
    bearer: Regex,
    jwt: Regex,
    url: Regex,
    tag: Regex,
    attribute: Regex,
}
fn patterns() -> &'static Patterns {
    static PATTERNS: OnceLock<Patterns> = OnceLock::new();
    PATTERNS.get_or_init(|| Patterns {
        quoted:Regex::new(r#"(?i)(?P<prefix>["']?(?P<key>[a-z_$][a-z0-9_$-]{0,79})["']?\s*[:=]\s*)(?P<value>"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*')"#).unwrap(),
        pair:Regex::new(r#"(?P<prefix>^|[?&#])(?P<key>[a-zA-Z0-9_%.-]{1,160})=(?P<value>[^&\s"'<>`#]*)"#).unwrap(),
        key:Regex::new(r"^[a-zA-Z0-9_%.-]{1,160}$").unwrap(),
        bearer:Regex::new(r"(?i)\bBearer\s+[A-Za-z0-9._~+/=-]+").unwrap(),
        jwt:Regex::new(r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b").unwrap(),
        url:Regex::new(r#"(?i)\bhttps?://[^\s"'<>`]+"#).unwrap(),
        tag:Regex::new(r#"(?is)<(?:meta|input)\b(?:[^"'>]|"[^"]*"|'[^']*')*>"#).unwrap(),
        attribute:Regex::new(r#"(?i)\b(?P<key>[a-z][a-z0-9_:-]*)\s*=\s*(?:"(?P<double>[^"]*)"|'(?P<single>[^']*)'|(?P<plain>[^\s"'=<>`]+))"#).unwrap(),
    })
}
fn decoded_pair(key: &str, value: &str) -> (String, String) {
    let mut url = tauri::Url::parse("https://redaction.invalid/").expect("fixed URL");
    url.set_query(Some(&format!("{key}={value}")));
    url.query_pairs()
        .next()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .unwrap_or_default()
}
fn clean_form(text: &str) -> Option<String> {
    let mut changed = false;
    let mut output = Vec::new();
    for item in text.split('&') {
        let (key, value) = item.split_once('=')?;
        if !patterns().key.is_match(key) {
            return None;
        }
        let (decoded_key, decoded_value) = decoded_pair(key, value);
        if credential_value(&decoded_key, &decoded_value) && decoded_value != EXCLUDED {
            output.push(format!("{key}=%5Bcredential_excluded%5D"));
            changed = true;
        } else {
            output.push(item.to_owned());
        }
    }
    changed.then(|| output.join("&"))
}
fn clean_text(text: &str) -> String {
    let patterns = patterns();
    let form = clean_form(text).unwrap_or_else(|| text.to_owned());
    let quoted = patterns
        .quoted
        .replace_all(&form, |captures: &Captures<'_>| {
            let value = &captures["value"];
            if credential_value(&captures["key"], &value[1..value.len() - 1]) {
                format!("{}\"{EXCLUDED}\"", &captures["prefix"])
            } else {
                captures[0].to_owned()
            }
        });
    let pairs = patterns
        .pair
        .replace_all(&quoted, |captures: &Captures<'_>| {
            let (key, value) = decoded_pair(&captures["key"], &captures["value"]);
            if credential_value(&key, &value) && value != EXCLUDED {
                format!(
                    "{}{}=%5Bcredential_excluded%5D",
                    &captures["prefix"], &captures["key"]
                )
            } else {
                captures[0].to_owned()
            }
        });
    let urls = patterns.url.replace_all(&pairs, |captures: &Captures<'_>| {
        if tauri::Url::parse(&captures[0])
            .is_ok_and(|u| !u.username().is_empty() || u.password().is_some())
        {
            "[credential_url_excluded]".to_owned()
        } else {
            captures[0].to_owned()
        }
    });
    let bearer = patterns.bearer.replace_all(&urls, EXCLUDED);
    let jwt = patterns.jwt.replace_all(&bearer, EXCLUDED);
    clean_html(&jwt)
}
fn attribute_value<'a>(captures: &Captures<'a>) -> &'a str {
    captures
        .name("double")
        .or_else(|| captures.name("single"))
        .or_else(|| captures.name("plain"))
        .map(|value| value.as_str())
        .unwrap_or("")
}
fn clean_html(text: &str) -> String {
    let patterns = patterns();
    patterns
        .tag
        .replace_all(text, |captures: &Captures<'_>| {
            let original = &captures[0];
            if original.len() > 8192 {
                return "[oversize_meta_input_excluded]".to_owned();
            }
            let names: Vec<String> = patterns
                .attribute
                .captures_iter(original)
                .filter_map(|attr| {
                    matches!(
                        attr["key"].to_ascii_lowercase().as_str(),
                        "name" | "property" | "type" | "id" | "http-equiv"
                    )
                    .then(|| normalized(attribute_value(&attr)))
                })
                .collect();
            patterns
                .attribute
                .replace_all(original, |attr: &Captures<'_>| {
                    let key = &attr["key"];
                    if matches!(key.to_ascii_lowercase().as_str(), "content" | "value")
                        && names.iter().any(|name| {
                            name.contains("csrf")
                                || name.contains("xsrf")
                                || credential_value(name, attribute_value(attr))
                        })
                    {
                        format!("{key}=\"{EXCLUDED}\"")
                    } else {
                        attr[0].to_owned()
                    }
                })
                .into_owned()
        })
        .into_owned()
}
pub fn clean_body(body: &str) -> Result<(String, bool), String> {
    if body.len() > BODY_LIMIT {
        return Err("body_too_large".into());
    }
    if json_keys::is_unambiguous_json(body).map_err(str::to_owned)? {
        let mut value = serde_json::from_str::<Value>(body).map_err(|_| "invalid_json_body")?;
        let before = value.clone();
        clean_value(&mut value, 0);
        if before != value {
            return Ok((
                serde_json::to_string(&value).map_err(|_| "invalid_body")?,
                true,
            ));
        }
        return Ok((body.to_owned(), false));
    }
    let clean = clean_text(body);
    let changed = clean != body;
    Ok((clean, changed))
}
pub fn safe_url(raw: &str) -> Option<String> {
    if raw.len() > 4096 {
        return None;
    }
    let mut url = tauri::Url::parse(raw).ok()?;
    if !url.username().is_empty() || url.password().is_some() {
        return None;
    }
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(key, value)| {
            let clean = if credential_value(&key, &value) {
                EXCLUDED.into()
            } else {
                clean_text(&value)
            };
            (key.into_owned(), clean)
        })
        .collect();
    url.set_fragment(None);
    url.set_query(None);
    if !pairs.is_empty() {
        url.query_pairs_mut().extend_pairs(pairs);
    }
    Some(url.into())
}
pub fn identity_path(raw: &str) -> bool {
    let Ok(url) = tauri::Url::parse(raw) else {
        return true;
    };
    url.path().split('/').any(|part| {
        matches!(
            part.to_ascii_lowercase().as_str(),
            "login"
                | "logout"
                | "oauth"
                | "oauth2"
                | "authorize"
                | "authentication"
                | "password"
                | "captcha"
                | "token"
        )
    })
}
pub fn clean_initiator(value: &Value) -> Option<Value> {
    let text = serde_json::to_string(value).ok()?;
    if text.len() > 8192 {
        return None;
    }
    let (clean, _) = clean_body(&text).ok()?;
    serde_json::from_str(&clean).ok()
}

#[cfg(test)]
#[path = "privacy_tests.rs"]
mod tests;
