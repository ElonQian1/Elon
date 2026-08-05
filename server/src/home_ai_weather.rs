//! 首页总 AI 的天气专用能力。
//!
//! 天气不能依赖普通网页搜索或模型猜测：由首页总 AI 判断是否需要天气工具，
//! 工具层负责校验地点并调用天气服务，最后用结构化数据生成简短回答。

use serde_json::{Map, Value};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use crate::{store::ConversationMessage, types::AppState};

const GEOCODING_URL: &str = "https://geocoding-api.open-meteo.com/v1/search";
const FORECAST_URL: &str = "https://api.open-meteo.com/v1/forecast";
const GEOCODING_HOST: &str = "geocoding-api.open-meteo.com";
const FORECAST_HOST: &str = "api.open-meteo.com";
const SOURCE_URL: &str = "https://open-meteo.com/en/docs";
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const WEATHER_REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WeatherDetail {
    Summary,
    HourlyRain,
    RainForecast,
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
        "天气预报",
        "降水",
        "雷雨",
        "阵雨",
        "雨",
    ]
    .iter()
    .any(|keyword| normalized.contains(keyword))
}

/// 判断用户是在询问降雨的具体时间，而不是只要一条天气概况。
/// 这里只负责选择更精确的小时预报数据，回答内容仍由结构化天气数据生成。
pub(crate) fn is_hourly_weather_request(message: &str) -> bool {
    let compact: String = message.chars().filter(|ch| !ch.is_whitespace()).collect();
    if compact.is_empty() {
        return false;
    }
    let asks_time = ["几点", "什么时候", "何时", "多久", "时段", "下到", "停雨"]
        .iter()
        .any(|marker| compact.contains(marker));
    asks_time
        && (compact.contains("雨")
            || compact.contains("降水")
            || compact.contains("下")
            || compact.contains("雷阵雨"))
}

/// 识别“未来几天哪天可能下雨”这一类多日降雨问题。
/// 这是工具参数缺失时的兜底，不负责解析地点，也不替代首页总 AI 的意图判断。
pub(crate) fn is_rain_forecast_request(message: &str) -> bool {
    let compact: String = message.chars().filter(|ch| !ch.is_whitespace()).collect();
    if compact.is_empty() || !(compact.contains('雨') || compact.contains("降水")) {
        return false;
    }
    [
        "未来",
        "哪天",
        "哪一天",
        "最近几天",
        "接下来几天",
        "几天内",
        "一周内",
        "这周",
        "本周",
        "下周",
    ]
    .iter()
    .any(|marker| compact.contains(marker))
}

pub(crate) fn is_weather_request_with_history(
    message: &str,
    history: &[ConversationMessage],
) -> bool {
    is_weather_request(message)
        || is_weather_location_follow_up(message, history)
        || is_weather_context_follow_up(message, history)
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
                .find_map(|item| {
                    extract_location(&item.content).or_else(|| {
                        if has_weather_context(history) {
                            extract_standalone_location(&item.content)
                        } else {
                            None
                        }
                    })
                })
        })
        .or_else(|| {
            history
                .iter()
                .rev()
                .filter(|item| item.role == "assistant")
                .find_map(|item| extract_location_from_weather_answer(&item.content))
        })
}

/// Tool 参数来自模型，不能把完整自然语言问题当作地名交给地理编码服务。
/// 这里只做安全边界校验，语言理解仍由首页总 AI 完成。
pub(crate) fn is_location_candidate(candidate: &str) -> bool {
    clean_location_candidate(candidate).is_some()
}

/// 返回当前问题对应的预报日：今天为 0，明天为 1，后天为 2。
pub(crate) fn day_offset(message: &str) -> usize {
    if message.contains("后天") {
        2
    } else if message.contains("明天") || message.contains("明日") {
        1
    } else {
        0
    }
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
        "雨",
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
        || candidate.contains('哪')
        || candidate.contains("什么")
        || candidate.contains("何时")
        || ["未来", "最近", "接下来", "下周", "本周", "会下"]
            .iter()
            .any(|marker| candidate.contains(marker))
        || candidate.chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }
    Some(candidate)
}

fn extract_standalone_location(message: &str) -> Option<String> {
    let compact = message
        .trim()
        .trim_matches(|ch: char| "，。？！?：:、".contains(ch));
    if compact.is_empty()
        || compact.chars().any(char::is_whitespace)
        || [
            "今天",
            "现在",
            "明天",
            "明日",
            "后天",
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
        .any(|value| compact.contains(value))
        || compact.ends_with(['呢', '吗'])
    {
        return None;
    }
    clean_location_candidate(compact)
}

fn is_weather_location_follow_up(message: &str, history: &[ConversationMessage]) -> bool {
    let compact = message
        .trim()
        .trim_matches(|ch: char| "，。？！?：:、".contains(ch));
    let length = compact.chars().count();
    if !(2..=20).contains(&length)
        || compact.chars().any(|ch| ch.is_whitespace())
        || ["今天", "现在", "明天", "明日", "后天"]
            .iter()
            .any(|marker| compact.contains(marker))
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

fn has_weather_context(history: &[ConversationMessage]) -> bool {
    history.iter().any(|item| {
        item.role == "assistant"
            && (item.content.contains(WEATHER_LOCATION_PROMPT)
                || extract_location_from_weather_answer(&item.content).is_some())
    })
}

fn is_weather_context_follow_up(message: &str, history: &[ConversationMessage]) -> bool {
    let compact = message
        .trim()
        .trim_matches(|ch: char| "，。？！?：:、".contains(ch));
    if compact.is_empty() {
        return false;
    }
    let relative_day = ["今天", "现在", "明天", "明日", "后天"]
        .iter()
        .any(|marker| compact.contains(marker));
    if !relative_day && !is_hourly_weather_request(&compact) {
        return false;
    }
    history.iter().rev().any(|item| {
        item.role == "assistant" && extract_location_from_weather_answer(&item.content).is_some()
    })
}

fn extract_location_from_weather_answer(message: &str) -> Option<String> {
    let looks_like_structured_weather = message.contains("数据更新时间：")
        || message.contains("预报日期：")
        || message.contains("按小时预报")
        || (message.contains("℃")
            && (message.contains("当前")
                || (message.contains("最高") && message.contains("最低"))));
    if !looks_like_structured_weather {
        return None;
    }
    ["今天", "明天", "后天"]
        .iter()
        .filter_map(|marker| message.find(marker))
        .min()
        .and_then(|index| clean_location_candidate(&message[..index]))
}

pub(crate) async fn lookup(
    state: &AppState,
    location: &str,
    day_offset: usize,
    hourly_detail: bool,
) -> WeatherLookup {
    let detail = if hourly_detail {
        WeatherDetail::HourlyRain
    } else {
        WeatherDetail::Summary
    };
    lookup_with_detail(state, location, day_offset, detail, day_offset + 1).await
}

pub(crate) async fn lookup_with_detail(
    state: &AppState,
    location: &str,
    day_offset: usize,
    detail: WeatherDetail,
    forecast_days: usize,
) -> WeatherLookup {
    let day_offset = day_offset.min(2);
    let forecast_days = forecast_days.clamp(day_offset + 1, 7);
    let cache_key = format!(
        "{}|{}|{:?}|{}",
        location.trim().to_lowercase(),
        day_offset,
        detail,
        forecast_days,
    );
    if let Ok(mut entries) = cache().lock() {
        if let Some(entry) = entries.get(&cache_key) {
            if entry.expires_at > Instant::now() {
                return WeatherLookup::Answer(entry.answer.clone());
            }
        }
        entries.retain(|_, entry| entry.expires_at > Instant::now());
    }

    let weather_client = build_weather_client(&state.http_client).await;
    let geo_response = match send_with_retry(
        &weather_client,
        GEOCODING_URL,
        vec![
            ("name", location.to_string()),
            ("count", "1".to_string()),
            ("language", "zh".to_string()),
            ("format", "json".to_string()),
        ],
        "geocoding",
    )
    .await
    {
        Some(response) => response,
        None => {
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

    let forecast_response = match send_with_retry(
        &weather_client,
        FORECAST_URL,
        vec![
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
            (
                "hourly",
                "precipitation_probability,rain,showers,weather_code".to_string(),
            ),
            ("forecast_days", forecast_days.to_string()),
            ("timezone", "auto".to_string()),
        ],
        "forecast",
    )
    .await
    {
        Some(response) => response,
        None => {
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
    let current = payload["current"].as_object();
    let daily = payload["daily"].as_object();
    let weather_code = daily
        .and_then(|value| value["weather_code"].as_array())
        .and_then(|values| values.get(day_offset))
        .and_then(Value::as_i64)
        .or_else(|| current.and_then(|value| value["weather_code"].as_i64()));
    let max_temperature = daily
        .and_then(|value| value["temperature_2m_max"].as_array())
        .and_then(|values| values.get(day_offset))
        .and_then(Value::as_f64);
    let min_temperature = daily
        .and_then(|value| value["temperature_2m_min"].as_array())
        .and_then(|values| values.get(day_offset))
        .and_then(Value::as_f64);
    let precipitation_probability = daily
        .and_then(|value| value["precipitation_probability_max"].as_array())
        .and_then(|values| values.get(day_offset))
        .and_then(Value::as_f64);
    let forecast_date = daily
        .and_then(|value| value["time"].as_array())
        .and_then(|values| values.get(day_offset))
        .and_then(Value::as_str);
    if detail == WeatherDetail::RainForecast {
        if let Some(reply) = rain_forecast_reply(&display_location, daily, forecast_days) {
            let answer = WeatherAnswer {
                reply,
                source_title: "Open-Meteo 多日天气数据".to_string(),
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
            return WeatherLookup::Answer(answer);
        }
        return WeatherLookup::Unavailable {
            location: display_location,
        };
    }

    let Some(weather_code) = weather_code else {
        return WeatherLookup::Unavailable {
            location: display_location,
        };
    };

    let period = match day_offset {
        1 => "明天",
        2 => "后天",
        _ => "今天",
    };
    if detail == WeatherDetail::HourlyRain {
        if let Some(reply) = hourly_rain_reply(
            &display_location,
            period,
            forecast_date.unwrap_or_default(),
            current.and_then(|value| value["time"].as_str()),
            payload["hourly"].as_object(),
        ) {
            let answer = WeatherAnswer {
                reply,
                source_title: "Open-Meteo 小时天气数据".to_string(),
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
            return WeatherLookup::Answer(answer);
        }
    }

    let condition = weather_description(weather_code);
    let mut reply = if day_offset == 0 {
        let Some(temperature) = current.and_then(|value| value["temperature_2m"].as_f64()) else {
            return WeatherLookup::Unavailable {
                location: display_location,
            };
        };
        let mut reply = format!("{display_location}{period}{condition}，当前 {temperature:.1}℃");
        if let Some(value) = current.and_then(|value| value["apparent_temperature"].as_f64()) {
            reply.push_str(&format!("，体感 {value:.1}℃"));
        }
        if let Some(wind) = current.and_then(|value| value["wind_speed_10m"].as_f64()) {
            reply.push_str(&format!("，风速 {wind:.1} km/h"));
        }
        reply
    } else {
        format!("{display_location}{period}{condition}")
    };
    if let (Some(min), Some(max)) = (min_temperature, max_temperature) {
        reply.push_str(&format!("。最高 {max:.1}℃，最低 {min:.1}℃"));
    }
    if let Some(probability) = precipitation_probability {
        reply.push_str(&format!("，降雨概率 {:.0}%", probability.clamp(0.0, 100.0)));
    }
    if day_offset == 0 {
        if let Some(updated_at) = current.and_then(|value| value["time"].as_str()) {
            reply.push_str(&format!("。数据更新时间：{updated_at}"));
        }
    } else if let Some(forecast_date) = forecast_date {
        reply.push_str(&format!("。预报日期：{forecast_date}"));
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

fn rain_forecast_reply(
    display_location: &str,
    daily: Option<&Map<String, Value>>,
    forecast_days: usize,
) -> Option<String> {
    let daily = daily?;
    let dates = daily.get("time")?.as_array()?;
    let probabilities = daily
        .get("precipitation_probability_max")
        .and_then(Value::as_array);
    let weather_codes = daily.get("weather_code").and_then(Value::as_array);
    let mut rainy_dates = Vec::new();

    for index in 0..dates.len().min(forecast_days) {
        let date = dates.get(index)?.as_str()?;
        let probability = value_at(probabilities, index).unwrap_or_default();
        let weather_code = value_at(weather_codes, index).unwrap_or_default() as i64;
        if probability >= 50.0 || matches!(weather_code, 51..=67 | 80..=82 | 95..=99) {
            rainy_dates.push(format_forecast_date(date));
        }
    }

    if rainy_dates.is_empty() {
        Some(format!(
            "{display_location}未来 {forecast_days} 天暂未发现明显降雨，出行前仍建议查看临近预报。"
        ))
    } else {
        Some(format!(
            "{display_location}未来 {forecast_days} 天预计可能在 {} 出现降雨。",
            rainy_dates.join("、")
        ))
    }
}

fn format_forecast_date(date: &str) -> String {
    let mut parts = date.split('-');
    let _year = parts.next();
    let month = parts.next().and_then(|value| value.parse::<u32>().ok());
    let day = parts.next().and_then(|value| value.parse::<u32>().ok());
    match (month, day) {
        (Some(month), Some(day)) => format!("{month}月{day}日"),
        _ => date.to_string(),
    }
}

fn hourly_rain_reply(
    display_location: &str,
    period: &str,
    forecast_date: &str,
    current_time: Option<&str>,
    hourly: Option<&Map<String, Value>>,
) -> Option<String> {
    let hourly = hourly?;
    let times = hourly.get("time")?.as_array()?;
    let probabilities = hourly
        .get("precipitation_probability")
        .and_then(Value::as_array);
    let rain = hourly.get("rain").and_then(Value::as_array);
    let showers = hourly.get("showers").and_then(Value::as_array);
    let weather_codes = hourly.get("weather_code").and_then(Value::as_array);
    let mut target_hours = Vec::new();

    for (index, time) in times.iter().enumerate() {
        let Some(time) = time.as_str() else {
            continue;
        };
        if !forecast_date.is_empty() && !time.starts_with(forecast_date) {
            continue;
        }
        if forecast_date != ""
            && current_time.is_some_and(|current_time| {
                current_time.starts_with(forecast_date) && time <= current_time
            })
        {
            continue;
        }
        let probability = value_at(probabilities, index)
            .unwrap_or(0.0)
            .clamp(0.0, 100.0);
        let rain_amount = value_at(rain, index).unwrap_or(0.0);
        let shower_amount = value_at(showers, index).unwrap_or(0.0);
        let weather_code = value_at(weather_codes, index).unwrap_or_default() as i64;
        let rainy = probability >= 50.0
            || rain_amount > 0.01
            || shower_amount > 0.01
            || matches!(weather_code, 51..=67 | 80..=82 | 95..=99);
        let label = time
            .split_once('T')
            .map(|(_, clock)| clock.get(..5).unwrap_or(clock))
            .unwrap_or(time)
            .to_string();
        target_hours.push((label, rainy, probability));
    }

    if target_hours.is_empty() {
        return None;
    }

    let mut intervals: Vec<(String, String, f64)> = Vec::new();
    let mut active_start: Option<usize> = None;
    let mut active_probability: f64 = 0.0;
    for (index, (_, rainy, probability)) in target_hours.iter().enumerate() {
        if *rainy {
            if active_start.is_none() {
                active_start = Some(index);
                active_probability = 0.0;
            }
            active_probability = active_probability.max(*probability);
        } else if let Some(start) = active_start.take() {
            intervals.push((
                target_hours[start].0.clone(),
                target_hours[index - 1].0.clone(),
                active_probability,
            ));
        }
    }
    if let Some(start) = active_start {
        intervals.push((
            target_hours[start].0.clone(),
            target_hours[target_hours.len() - 1].0.clone(),
            active_probability,
        ));
    }

    if intervals.is_empty() {
        return Some(format!(
            "{display_location}{period}按小时预报暂未发现明显降雨时段，出行前仍建议留意临近预报。"
        ));
    }

    let ranges = intervals
        .iter()
        .map(|(start, end, _)| {
            if start == end {
                start.clone()
            } else {
                format!("{start}—{end}")
            }
        })
        .collect::<Vec<_>>()
        .join("、");
    let max_probability = intervals
        .iter()
        .map(|(_, _, probability)| *probability)
        .fold(0.0, f64::max);
    let probability_note = if max_probability > 0.0 {
        format!("，对应时段降雨概率最高约 {:.0}%", max_probability)
    } else {
        String::new()
    };
    Some(format!(
        "{display_location}{period}按小时预报，预计可能在 {ranges} 出现降雨{probability_note}。"
    ))
}

fn value_at(values: Option<&Vec<Value>>, index: usize) -> Option<f64> {
    values?.get(index).and_then(Value::as_f64)
}

async fn build_weather_client(fallback: &reqwest::Client) -> reqwest::Client {
    let geocoding_ipv4 = resolve_ipv4(GEOCODING_HOST).await;
    let forecast_ipv4 = resolve_ipv4(FORECAST_HOST).await;
    let mut builder = reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(Duration::from_secs(4))
        .timeout(WEATHER_REQUEST_TIMEOUT);
    if let Some(address) = geocoding_ipv4 {
        builder = builder.resolve(GEOCODING_HOST, address);
    }
    if let Some(address) = forecast_ipv4 {
        builder = builder.resolve(FORECAST_HOST, address);
    }
    builder.build().unwrap_or_else(|_| fallback.clone())
}

async fn resolve_ipv4(host: &str) -> Option<SocketAddr> {
    tokio::net::lookup_host((host, 443))
        .await
        .ok()?
        .find(|address| address.is_ipv4())
}

async fn send_with_retry(
    client: &reqwest::Client,
    url: &str,
    query: Vec<(&str, String)>,
    phase: &str,
) -> Option<reqwest::Response> {
    for attempt in 1..=2 {
        match client.get(url).query(&query).send().await {
            Ok(response) if response.status().is_success() => return Some(response),
            Ok(response) => {
                tracing::warn!(
                    phase,
                    attempt,
                    status = %response.status(),
                    "天气服务返回非成功状态"
                );
            }
            Err(error) => {
                tracing::warn!(phase, attempt, error = %error, "天气服务请求失败");
            }
        }
        if attempt < 2 {
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
    }
    None
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
    include!("home_ai_weather_tests.rs");
}
