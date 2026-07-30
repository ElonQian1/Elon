//! 首页总 AI 的天气专用能力。
//!
//! 天气不能依赖普通网页搜索或模型猜测：先确定地点，再调用天气服务，
//! 最后用结构化数据生成简短回答。地点不明确时直接追问，不进入模型链路。

use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use crate::{store::ConversationMessage, types::AppState};

const GEOCODING_URL: &str = "https://geocoding-api.open-meteo.com/v1/search";
const FORECAST_URL: &str = "https://api.open-meteo.com/v1/forecast";
const SOURCE_URL: &str = "https://open-meteo.com/en/docs";
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const WEATHER_LOCATION_PROMPT: &str =
    "可以，我能帮你查询实时天气。你想查询哪个城市或地区？例如北京、上海。";

#[derive(Debug, Clone)]
pub(crate) struct WeatherAnswer {
    pub(crate) reply: String,
    pub(crate) source_title: String,
    pub(crate) source_url: String,
}

#[derive(Debug, Clone)]
pub(crate) enum WeatherLookup {
    Answer(WeatherAnswer),
    NotFound { location: String },
    Unavailable { location: String },
}

#[derive(Debug, Clone)]
struct CachedWeather {
    expires_at: Instant,
    answer: WeatherAnswer,
}

static WEATHER_CACHE: OnceLock<Mutex<HashMap<String, CachedWeather>>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<String, CachedWeather>> {
    WEATHER_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn is_weather_request(message: &str) -> bool {
    let normalized = message.trim().to_lowercase();
    if normalized.is_empty() {
        return false;
    }
    [
        "天气",
        "气温",
        "温度",
        "下雨",
        "降雨",
        "降雪",
        "下雪",
        "空气质量",
        "天气预警",
    ]
    .iter()
    .any(|keyword| normalized.contains(keyword))
}

pub(crate) fn is_weather_request_with_history(
    message: &str,
    history: &[ConversationMessage],
) -> bool {
    is_weather_request(message) || is_weather_location_follow_up(message, history)
}

/// 从当前问题或近期用户消息中解析地点。没有明确地点时返回 None，
/// 让上层向用户追问，而不是猜测所在地。
pub(crate) fn resolve_location(message: &str, history: &[ConversationMessage]) -> Option<String> {
    extract_location(message)
        .or_else(|| {
            if is_weather_location_follow_up(message, history) {
                clean_location_candidate(message)
            } else {
                None
            }
        })
        .or_else(|| {
            history
                .iter()
                .rev()
                .filter(|item| item.role == "user")
                .find_map(|item| extract_location(&item.content))
        })
}

pub(crate) fn missing_location_reply() -> &'static str {
    WEATHER_LOCATION_PROMPT
}

pub(crate) fn not_found_reply(location: &str) -> String {
    format!("我没有找到“{location}”这个地点。请换成更具体的城市或地区名称，我再帮你查询。")
}

pub(crate) fn unavailable_reply(location: &str) -> String {
    format!("暂时无法获取{location}的实时天气，请稍后重试。")
}

pub(crate) fn extract_location(message: &str) -> Option<String> {
    let normalized = message
        .trim()
        .trim_end_matches(['?', '？', '。', '！', '!'])
        .replace('，', ",")
        .replace('：', ":");
    if normalized.is_empty() {
        return None;
    }
    if !is_weather_request(&normalized) {
        for prefix in ["我在", "我人在", "位于", "位置是", "常住地是"] {
            if let Some(candidate) = normalized.strip_prefix(prefix) {
                return clean_location_candidate(candidate);
            }
        }
        return None;
    }

    let marker = [
        "天气预警",
        "空气质量",
        "天气",
        "气温",
        "温度",
        "降雨",
        "降雪",
        "下雨",
        "下雪",
    ]
    .iter()
    .filter_map(|marker| normalized.find(marker).map(|index| (index, *marker)))
    .min_by_key(|(index, _)| *index);
    let Some((index, _)) = marker else {
        return None;
    };

    let mut candidate = normalized[..index].to_string();
    for prefix in [
        "请问",
        "请查",
        "请查询",
        "帮我查一下",
        "帮我查下",
        "帮我查",
        "查一下",
        "查询",
        "搜索",
        "告诉我",
        "我想知道",
        "我在",
    ] {
        candidate = candidate.trim_start_matches(prefix).trim().to_string();
    }
    if let Some(after) = candidate.rsplit_once("在").map(|(_, after)| after) {
        candidate = after.trim().to_string();
    }
    for noise in [
        "今天",
        "明天",
        "后天",
        "现在",
        "当前",
        "怎么样",
        "如何",
        "会不会",
        "会",
        "吗",
        "呢",
        "的",
    ] {
        candidate = candidate.replace(noise, "");
    }
    candidate = candidate
        .split([',', ':', ' '])
        .last()
        .unwrap_or_default()
        .trim()
        .to_string();
    candidate = candidate
        .trim_matches(|ch: char| "，。？！?：:、".contains(ch))
        .to_string();

    clean_location_candidate(&candidate)
}

fn clean_location_candidate(candidate: &str) -> Option<String> {
    let candidate = candidate
        .split([',', ':', ' ', '，', '。', '！', '？', '?'])
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    let length = candidate.chars().count();
    if !(2..=32).contains(&length)
        || candidate.contains("天气")
        || candidate.chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }
    Some(candidate)
}

fn is_weather_location_follow_up(message: &str, history: &[ConversationMessage]) -> bool {
    let compact = message
        .trim()
        .trim_matches(|ch: char| "，。？！?：:、".contains(ch));
    let length = compact.chars().count();
    if !(2..=20).contains(&length)
        || compact.chars().any(|ch| ch.is_whitespace())
        || [
            "你好",
            "您好",
            "嗨",
            "哈喽",
            "谢谢",
            "感谢",
            "好的",
            "明白了",
        ]
        .iter()
        .any(|value| compact == *value)
    {
        return false;
    }
    history.iter().rev().any(|item| {
        item.role == "assistant"
            && (item.content.contains(WEATHER_LOCATION_PROMPT)
                || (item.content.contains("天气") && item.content.contains("哪个城市")))
    })
}

pub(crate) async fn lookup(state: &AppState, location: &str) -> WeatherLookup {
    let cache_key = location.trim().to_lowercase();
    if let Ok(mut entries) = cache().lock() {
        if let Some(entry) = entries.get(&cache_key) {
            if entry.expires_at > Instant::now() {
                return WeatherLookup::Answer(entry.answer.clone());
            }
        }
        entries.retain(|_, entry| entry.expires_at > Instant::now());
    }

    let geo_response = match state
        .http_client
        .get(GEOCODING_URL)
        .query(&[
            ("name", location),
            ("count", "1"),
            ("language", "zh"),
            ("format", "json"),
        ])
        .timeout(Duration::from_secs(6))
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => response,
        _ => {
            return WeatherLookup::Unavailable {
                location: location.to_string(),
            }
        }
    };
    let geo_payload = match geo_response.json::<Value>().await {
        Ok(payload) => payload,
        Err(_) => {
            return WeatherLookup::Unavailable {
                location: location.to_string(),
            }
        }
    };
    let Some(place) = geo_payload["results"]
        .as_array()
        .and_then(|items| items.first())
    else {
        return WeatherLookup::NotFound {
            location: location.to_string(),
        };
    };
    let Some(latitude) = place["latitude"].as_f64() else {
        return WeatherLookup::Unavailable {
            location: location.to_string(),
        };
    };
    let Some(longitude) = place["longitude"].as_f64() else {
        return WeatherLookup::Unavailable {
            location: location.to_string(),
        };
    };
    let display_location = display_location(place, location);

    let forecast_response = match state
        .http_client
        .get(FORECAST_URL)
        .query(&[
            ("latitude", latitude.to_string()),
            ("longitude", longitude.to_string()),
            (
                "current",
                "temperature_2m,apparent_temperature,weather_code,wind_speed_10m".to_string(),
            ),
            (
                "daily",
                "weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max"
                    .to_string(),
            ),
            ("forecast_days", "1".to_string()),
            ("timezone", "auto".to_string()),
        ])
        .timeout(Duration::from_secs(6))
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => response,
        _ => {
            return WeatherLookup::Unavailable {
                location: display_location,
            }
        }
    };
    let payload = match forecast_response.json::<Value>().await {
        Ok(payload) => payload,
        Err(_) => {
            return WeatherLookup::Unavailable {
                location: display_location,
            }
        }
    };
    let Some(current) = payload["current"].as_object() else {
        return WeatherLookup::Unavailable {
            location: display_location,
        };
    };
    let daily = payload["daily"].as_object();
    let temperature = current["temperature_2m"].as_f64();
    let apparent_temperature = current["apparent_temperature"].as_f64();
    let weather_code = current["weather_code"].as_i64();
    let wind_speed = current["wind_speed_10m"].as_f64();
    let max_temperature = daily
        .and_then(|value| value["temperature_2m_max"].as_array())
        .and_then(|values| values.first())
        .and_then(Value::as_f64);
    let min_temperature = daily
        .and_then(|value| value["temperature_2m_min"].as_array())
        .and_then(|values| values.first())
        .and_then(Value::as_f64);
    let precipitation_probability = daily
        .and_then(|value| value["precipitation_probability_max"].as_array())
        .and_then(|values| values.first())
        .and_then(Value::as_f64);
    let Some(temperature) = temperature else {
        return WeatherLookup::Unavailable {
            location: display_location,
        };
    };

    let condition = weather_code.map(weather_description).unwrap_or("天气情况");
    let mut reply = format!("{display_location}今天{condition}，当前 {temperature:.1}℃");
    if let Some(value) = apparent_temperature {
        reply.push_str(&format!("，体感 {value:.1}℃"));
    }
    if let (Some(min), Some(max)) = (min_temperature, max_temperature) {
        reply.push_str(&format!("。最高 {max:.1}℃，最低 {min:.1}℃"));
    }
    if let Some(probability) = precipitation_probability {
        reply.push_str(&format!("，降雨概率 {:.0}%", probability.clamp(0.0, 100.0)));
    }
    if let Some(wind) = wind_speed {
        reply.push_str(&format!("，风速 {wind:.1} km/h"));
    }
    if let Some(updated_at) = current["time"].as_str() {
        reply.push_str(&format!("。数据更新时间：{updated_at}"));
    }
    reply.push('。');

    let answer = WeatherAnswer {
        reply,
        source_title: "Open-Meteo 天气数据".to_string(),
        source_url: SOURCE_URL.to_string(),
    };
    if let Ok(mut entries) = cache().lock() {
        entries.insert(
            cache_key,
            CachedWeather {
                expires_at: Instant::now() + CACHE_TTL,
                answer: answer.clone(),
            },
        );
    }
    WeatherLookup::Answer(answer)
}

fn display_location(place: &Value, fallback: &str) -> String {
    let name = place["name"].as_str().unwrap_or(fallback).trim();
    let admin1 = place["admin1"].as_str().unwrap_or("").trim();
    if !admin1.is_empty() && admin1 != name && !name.contains(admin1) {
        format!("{admin1}{name}")
    } else {
        name.to_string()
    }
}

fn weather_description(code: i64) -> &'static str {
    match code {
        0 => "晴",
        1..=3 => "多云",
        45 | 48 => "有雾",
        51..=57 => "有毛毛雨",
        61..=67 => "有雨",
        71..=77 => "有雪",
        80..=82 => "有阵雨",
        85 | 86 => "有阵雪",
        95 => "有雷雨",
        96 | 99 => "有雷雨和冰雹",
        _ => "天气情况",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history(items: &[(&str, &str)]) -> Vec<ConversationMessage> {
        items
            .iter()
            .map(|(role, content)| ConversationMessage {
                role: (*role).to_string(),
                content: (*content).to_string(),
            })
            .collect()
    }

    #[test]
    fn detects_weather_questions() {
        assert!(is_weather_request("北京今天天气怎么样"));
        assert!(is_weather_request("上海明天会下雨吗"));
        assert!(!is_weather_request("今天星期几"));
    }

    #[test]
    fn extracts_explicit_location() {
        assert_eq!(
            extract_location("北京今天天气怎么样").as_deref(),
            Some("北京")
        );
        assert_eq!(
            extract_location("帮我查一下上海的天气").as_deref(),
            Some("上海")
        );
        assert_eq!(
            extract_location("上海明天会下雨吗").as_deref(),
            Some("上海")
        );
        assert_eq!(extract_location("今天天气怎么样").as_deref(), None);
    }

    #[test]
    fn reuses_recent_user_location() {
        let recent = history(&[("user", "我人在杭州"), ("assistant", "好的")]);
        assert_eq!(
            resolve_location("今天会下雨吗", &recent).as_deref(),
            Some("杭州")
        );
    }

    #[test]
    fn treats_city_only_reply_as_weather_follow_up() {
        let recent = history(&[("assistant", WEATHER_LOCATION_PROMPT)]);
        assert!(is_weather_request_with_history("广州", &recent));
        assert_eq!(resolve_location("广州", &recent).as_deref(), Some("广州"));
    }

    #[test]
    fn maps_weather_codes() {
        assert_eq!(weather_description(0), "晴");
        assert_eq!(weather_description(95), "有雷雨");
    }
}
