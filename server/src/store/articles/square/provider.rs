//! Fixed official OpenAPI transport. Never forwards keys to media hosts or follows redirects.
use super::*;
use std::time::Duration;
const API: &str = "https://www.binance.com/bapi/composite";
pub(super) struct Client {
    http: reqwest::Client,
}
pub(super) enum Outcome {
    Published(String),
    Failed(String),
    Uncertain,
}
impl Client {
    pub fn new() -> Result<Self> {
        Ok(Self {
            http: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(60))
                .build()?,
        })
    }
    async fn call(&self, key: &str, version: u8, path: &str, body: Value) -> Result<(u16, Value)> {
        let mut response = self
            .http
            .post(format!("{API}/v{version}/public/pgc/openApi{path}"))
            .header("X-Square-OpenAPI-Key", key)
            .header("clienttype", "binanceSkill")
            .json(&body)
            .send()
            .await
            .map_err(|_| fail(502, "币安连接失败或超时"))?;
        let status = response.status().as_u16();
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| fail(502, "币安回执读取失败"))?
        {
            if bytes.len() + chunk.len() > 64 * 1024 {
                return Err(fail(502, "币安回执超过大小限制"));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok((
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        ))
    }
    async fn data(&self, key: &str, path: &str, body: Value) -> Result<Value> {
        let (status, value) = self.call(key, 2, path, body).await?;
        if !(200..300).contains(&status) || value["code"] != "000000" {
            return Err(fail(502, &failure(&value)));
        }
        Ok(value["data"].clone())
    }
    pub async fn upload(
        &self,
        key: &str,
        mime: &str,
        bytes: Vec<u8>,
        video: bool,
    ) -> Result<(String, String)> {
        let suffix = match mime {
            "image/png" => "png",
            "image/webp" => "webp",
            "video/mp4" => "mp4",
            "video/webm" => "webm",
            _ => "jpg",
        };
        let name = format!("{}.{}", uuid::Uuid::new_v4(), suffix);
        let ticket = if video {
            self.data(
                key,
                "/video/preSign",
                json!({"fileName":name,"size":bytes.len()}),
            )
            .await?
        } else {
            self.data(key, "/image/presignedUrl", json!({"imageName":name}))
                .await?
        };
        let url = ticket["presignedUrl"]
            .as_str()
            .filter(|u| trusted_media_url(u))
            .ok_or_else(|| fail(502, "币安返回了不受支持的媒体上传地址"))?;
        let ticket_id = ticket["fileTicket"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 1024)
            .ok_or_else(|| fail(502, "币安媒体上传凭据无效"))?
            .to_string();
        let response = self
            .http
            .put(url)
            .header("content-type", mime)
            .body(bytes)
            .send()
            .await
            .map_err(|_| fail(502, "媒体上传失败，请重试"))?;
        if !response.status().is_success() {
            return Err(fail(502, "媒体上传未完成，请重试"));
        }
        for _ in 0..10 {
            let state = self
                .data(key, "/image/imageStatus", json!({"fileTicket":ticket_id}))
                .await?;
            match state["status"].as_i64() {
                Some(1) => {
                    let url = if video {
                        String::new()
                    } else {
                        state["imageUrl"]
                            .as_str()
                            .filter(|u| trusted_media_url(u))
                            .ok_or_else(|| fail(502, "媒体处理结果无效"))?
                            .to_string()
                    };
                    return Ok((ticket_id, url));
                }
                Some(2) => return Err(fail(400, "币安未能处理该媒体，请更换素材后重试")),
                _ => tokio::time::sleep(Duration::from_secs(3)).await,
            }
        }
        Err(fail(504, "媒体仍在处理，尚未发帖；稍后可重试"))
    }
    pub async fn publish(&self, key: &str, body: Value) -> Outcome {
        match self.call(key, 1, "/content/add", body).await {
            Ok((status, value)) => classify(status, &value),
            Err(_) => Outcome::Uncertain,
        }
    }
}
pub(super) fn trusted_media_url(raw: &str) -> bool {
    if raw.len() > 16_384 {
        return false;
    }
    reqwest::Url::parse(raw).ok().is_some_and(|u| {
        u.scheme() == "https"
            && u.username().is_empty()
            && u.password().is_none()
            && u.port_or_known_default() == Some(443)
            && u.fragment().is_none()
            && u.host_str().is_some_and(|h| {
                ["amazonaws.com", "bnbstatic.com", "binance.com"]
                    .iter()
                    .any(|d| h == *d || h.ends_with(&format!(".{d}")))
            })
    })
}
pub(super) fn classify(status: u16, value: &Value) -> Outcome {
    if status >= 500 || value.is_null() {
        return Outcome::Uncertain;
    }
    if value["code"] == "000000" {
        if !(200..300).contains(&status) {
            return Outcome::Uncertain;
        }
        let id = value["data"]["id"]
            .as_str()
            .map(String::from)
            .or_else(|| value["data"]["id"].as_u64().map(|n| n.to_string()));
        return match id.filter(|id| post_link(id).is_some()) {
            Some(id) => Outcome::Published(id),
            None => Outcome::Uncertain,
        };
    }
    if value["code"]
        .as_str()
        .is_some_and(|s| !s.is_empty() && s.len() <= 16 && s.bytes().all(|c| c.is_ascii_digit()))
    {
        Outcome::Failed(failure(value))
    } else {
        Outcome::Uncertain
    }
}
fn failure(value: &Value) -> String {
    let code = value["code"].as_str().unwrap_or("");
    let message = match code {
        "220003" | "220004" => "发帖凭证不存在或已过期，请重新绑定",
        "220009" => "币安今日发帖额度已用完，请稍后重试",
        "220014" => "币安今日媒体上传额度已用完，请稍后重试",
        "20002" | "20022" => "币安内容审核未通过，请修改文章后重新预览",
        "20013" => "内容超出币安长度限制，请缩短文章后重试",
        "20020" | "220011" => "币安未接受空白正文",
        "30008" | "2000001" | "2000002" => "账号暂不能发帖，请前往币安创作者中心查看",
        _ => "币安未接受请求，请检查账号和内容后重试",
    };
    // Do not expose upstream text: it can echo credentials or private content.
    if !code.is_empty() && code.len() <= 16 && code.bytes().all(|c| c.is_ascii_digit()) {
        format!("{message}（{code}）")
    } else {
        message.into()
    }
}
