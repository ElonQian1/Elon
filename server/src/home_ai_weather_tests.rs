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
    assert!(is_weather_request("雨几点下到几点"));
    assert!(!is_weather_request("今天星期几"));
}

#[test]
fn detects_hourly_rain_questions() {
    assert!(is_hourly_weather_request("雨几点下到几点"));
    assert!(is_hourly_weather_request("明天什么时候下雨"));
    assert!(is_hourly_weather_request("降雨时段"));
    assert!(!is_hourly_weather_request("今天温度怎么样"));
}

#[test]
fn detects_multi_day_rain_questions() {
    assert!(is_rain_forecast_request("未来哪一天会下雨"));
    assert!(is_rain_forecast_request("接下来几天有降水吗"));
    assert!(!is_rain_forecast_request("今天温度怎么样"));
    assert!(!is_rain_forecast_request("明天什么时候下雨"));
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
    assert_eq!(
        extract_location("广州雨几点下到几点").as_deref(),
        Some("广州")
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
fn reuses_location_from_successful_weather_answer() {
    let recent = history(&[(
        "assistant",
        "广东广州今天多云，当前 28.1℃。数据更新时间：2026-07-30T17:30。",
    )]);
    assert!(is_weather_request_with_history("明天天气情况", &recent));
    assert_eq!(
        resolve_location("明天天气情况", &recent).as_deref(),
        Some("广东广州")
    );
    assert_eq!(
        resolve_location("后天呢", &recent).as_deref(),
        Some("广东广州")
    );
}

#[test]
fn parses_forecast_day_from_follow_up() {
    assert_eq!(day_offset("今天天气"), 0);
    assert_eq!(day_offset("明天天气情况"), 1);
    assert_eq!(day_offset("后天呢"), 2);
}

#[test]
fn reuses_location_from_future_weather_answer() {
    let recent = history(&[(
        "assistant",
        "广东广州明天多云。最高 30.0℃，最低 25.0℃。预报日期：2026-07-31。",
    )]);
    assert_eq!(
        resolve_location("后天天气", &recent).as_deref(),
        Some("广东广州")
    );
}

#[test]
fn does_not_treat_relative_day_as_a_city() {
    let recent = history(&[("assistant", WEATHER_LOCATION_PROMPT)]);
    assert!(!is_weather_location_follow_up("明天呢", &recent));
    assert_eq!(extract_standalone_location("明天呢"), None);
    assert_eq!(extract_standalone_location("现在是什么情况"), None);
}

#[test]
fn ignores_stale_relative_day_when_restoring_location() {
    let recent = history(&[
        ("user", "广州"),
        ("assistant", WEATHER_LOCATION_PROMPT),
        ("user", "明天呢"),
    ]);
    assert_eq!(
        resolve_location("现在是什么情况", &recent).as_deref(),
        Some("广州")
    );
}

#[test]
fn restores_location_from_wrapped_weather_answer_and_prior_city() {
    let recent = history(&[
        ("user", "广州"),
        (
            "assistant",
            "广东广州今天多云，当前 28.1℃，体感 33.9℃。\n最高 28.9℃，最低 24.7℃。",
        ),
    ]);
    assert_eq!(
        extract_location_from_weather_answer(&recent[1].content).as_deref(),
        Some("广东广州")
    );
    assert_eq!(
        resolve_location("明天天气情况", &recent).as_deref(),
        Some("广州")
    );
    assert_eq!(resolve_location("后天呢", &recent).as_deref(), Some("广州"));
}

#[test]
fn maps_weather_codes() {
    assert_eq!(weather_description(0), "晴");
    assert_eq!(weather_description(95), "有雷雨");
}

#[test]
fn restores_location_for_hourly_rain_follow_up() {
    let recent = history(&[(
        "assistant",
        "广东广州今天有雷雨，当前 26.2℃，最高 28.7℃，最低 24.2℃。数据更新时间：2026-08-01T14:45。",
    )]);
    assert!(is_weather_request_with_history("雨几点下到几点", &recent));
    assert_eq!(
        resolve_location("雨几点下到几点", &recent).as_deref(),
        Some("广东广州")
    );
}

#[test]
fn does_not_treat_future_rain_question_as_a_location() {
    assert_eq!(extract_location("未来哪一天会下雨"), None);
    assert!(!is_location_candidate("未来哪一天会下雨"));
}

#[test]
fn restores_location_for_future_rain_follow_up() {
    let recent = history(&[(
        "assistant",
        "广东广州今天有雷雨，当前 26.2℃，最高 28.7℃，最低 24.2℃。数据更新时间：2026-08-01T14:45。",
    )]);
    assert!(is_weather_request_with_history("未来哪一天会下雨", &recent));
    assert_eq!(
        resolve_location("未来哪一天会下雨", &recent).as_deref(),
        Some("广东广州")
    );
}

#[test]
fn formats_multi_day_rain_forecast() {
    let daily = serde_json::json!({
        "time": ["2026-08-05", "2026-08-06", "2026-08-07"],
        "precipitation_probability_max": [20, 80, 60],
        "weather_code": [1, 61, 3]
    });
    let reply = rain_forecast_reply("广东广州", daily.as_object(), 3).expect("rain forecast");
    assert!(reply.contains("8月6日"));
    assert!(reply.contains("8月7日"));
}

#[test]
fn formats_hourly_rain_intervals() {
    let payload = serde_json::json!({
        "time": [
            "2026-08-01T14:00", "2026-08-01T15:00", "2026-08-01T16:00",
            "2026-08-01T17:00", "2026-08-01T18:00"
        ],
        "precipitation_probability": [10, 60, 90, 70, 20],
        "rain": [0, 0, 0.3, 0.2, 0],
        "showers": [0, 0, 0, 0, 0],
        "weather_code": [1, 61, 61, 61, 1]
    });
    let reply = hourly_rain_reply("广东广州", "今天", "2026-08-01", None, payload.as_object())
        .expect("hourly reply");
    assert!(reply.contains("15:00—17:00"));
    assert!(reply.contains("90%"));
}
