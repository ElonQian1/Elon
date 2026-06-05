//! 微信支付 v3 核心工具
//!
//! 职责：
//! - RSA-SHA256（PKCS1v1.5）请求签名
//! - 调用微信支付 API（创建 App 订单、查询订单）
//! - AES-256-GCM 解密回调通知 resource 字段
//!
//! **配置方式（环境变量，均可在 .env 中设置）：**
//! ```
//! WECHAT_APP_ID=wx...          # 微信开放平台 AppID
//! WECHAT_MCH_ID=1234567890     # 商户号
//! WECHAT_SERIAL_NO=ABC...      # API 证书序列号（大写十六进制）
//! WECHAT_PRIVATE_KEY=-----BEGIN PRIVATE KEY-----\n...\n-----END PRIVATE KEY-----
//! WECHAT_API_V3_KEY=32位ASCII密钥   # APIv3 密钥（用于解密通知）
//! WECHAT_PAY_NOTIFY_URL=https://你的域名/api/pay/notify  # 需要 HTTPS
//! ```

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rsa::signature::{SignatureEncoding, Signer};
use rsa::{pkcs1v15::SigningKey, pkcs8::DecodePrivateKey, RsaPrivateKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

// ── 配置 ──────────────────────────────────────────────────────────────────────

/// 微信支付配置，从环境变量读取。
#[derive(Clone)]
pub struct WechatPayConfig {
    pub app_id: String,
    pub mch_id: String,
    pub serial_no: String,
    /// PKCS8 PEM 格式私钥（支持 \n 转义的单行或多行）
    pub private_key_pem: String,
    /// APIv3 密钥（32字节 ASCII）
    pub api_v3_key: String,
    /// 支付回调地址（需要 HTTPS）
    pub notify_url: String,
}

impl WechatPayConfig {
    /// 从环境变量加载配置，任意字段缺失均返回 None。
    pub fn from_env() -> Option<Self> {
        let app_id = std::env::var("WECHAT_APP_ID").ok()?;
        let mch_id = std::env::var("WECHAT_MCH_ID").ok()?;
        let serial_no = std::env::var("WECHAT_SERIAL_NO").ok()?;
        let private_key_pem = std::env::var("WECHAT_PRIVATE_KEY").ok()?;
        let api_v3_key = std::env::var("WECHAT_API_V3_KEY").ok()?;
        let notify_url = std::env::var("WECHAT_PAY_NOTIFY_URL")
            .unwrap_or_else(|_| "https://placeholder.example.com/api/pay/notify".to_string());
        // 允许环境变量中用 \n 字面量代替真正的换行符
        let private_key_pem = private_key_pem.replace("\\n", "\n");
        Some(Self {
            app_id,
            mch_id,
            serial_no,
            private_key_pem,
            api_v3_key,
            notify_url,
        })
    }
}

// ── 签名工具 ──────────────────────────────────────────────────────────────────

/// 生成随机 nonce（32位字符）
pub fn new_nonce() -> String {
    Uuid::new_v4().to_string().replace('-', "")
}

/// 当前 Unix 时间戳（秒）
pub fn timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 用商户私钥对消息进行 RSA-SHA256（PKCS1v1.5）签名，返回 Base64。
pub fn rsa_sign(private_key_pem: &str, message: &str) -> Result<String> {
    let key = RsaPrivateKey::from_pkcs8_pem(private_key_pem)
        .map_err(|e| anyhow!("解析微信支付私钥失败: {e}"))?;
    let signing_key = SigningKey::<Sha256>::new(key);
    let sig: rsa::pkcs1v15::Signature = signing_key.sign(message.as_bytes());
    Ok(B64.encode(sig.to_bytes()))
}

/// 构造微信支付 v3 请求的 Authorization 头。
///
/// 消息格式：`{method}\n{url_path_with_query}\n{timestamp}\n{nonce}\n{body}\n`
pub fn build_auth_header(
    cfg: &WechatPayConfig,
    method: &str,
    url_path: &str,
    body: &str,
) -> Result<String> {
    let ts = timestamp_secs();
    let nonce = new_nonce();
    let message = format!("{method}\n{url_path}\n{ts}\n{nonce}\n{body}\n");
    let sig = rsa_sign(&cfg.private_key_pem, &message)?;
    Ok(format!(
        r#"WECHATPAY2-SHA256-RSA2048 mchid="{mchid}",nonce_str="{nonce}",timestamp="{ts}",serial_no="{serial}",signature="{sig}""#,
        mchid = cfg.mch_id,
        serial = cfg.serial_no,
    ))
}

// ── AES-256-GCM 解密（微信回调 resource）────────────────────────────────────

/// 解密微信支付回调通知中的 resource 字段。
///
/// - `api_v3_key`：32字节 ASCII（直接作为 AES-256 密钥）
/// - `nonce`：12字节 IV（来自 resource.nonce_str）
/// - `associated_data`：附加数据（来自 resource.associated_data）
/// - `ciphertext_b64`：Base64 密文（来自 resource.ciphertext）
pub fn decrypt_notify_resource(
    api_v3_key: &str,
    nonce: &str,
    associated_data: &str,
    ciphertext_b64: &str,
) -> Result<String> {
    use aes_gcm::{
        aead::{Aead, KeyInit, Payload},
        Aes256Gcm, Key, Nonce,
    };

    let key_bytes = api_v3_key.as_bytes();
    if key_bytes.len() != 32 {
        return Err(anyhow!("WECHAT_API_V3_KEY 必须恰好为 32 字节 ASCII"));
    }
    let nonce_bytes = nonce.as_bytes();
    if nonce_bytes.len() != 12 {
        return Err(anyhow!(
            "nonce_str 长度必须为 12 字节，实际 {} 字节",
            nonce_bytes.len()
        ));
    }

    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);
    let ciphertext = B64.decode(ciphertext_b64)?;

    let plaintext = cipher
        .decrypt(
            nonce,
            Payload {
                msg: &ciphertext,
                aad: associated_data.as_bytes(),
            },
        )
        .map_err(|e| anyhow!("AES-GCM 解密失败: {:?}", e))?;

    Ok(String::from_utf8(plaintext)?)
}

// ── API 类型 ──────────────────────────────────────────────────────────────────

/// 向微信创建 App 支付订单的请求体。
#[derive(Serialize)]
struct CreateAppOrderRequest<'a> {
    appid: &'a str,
    mchid: &'a str,
    description: &'a str,
    out_trade_no: &'a str,
    notify_url: &'a str,
    amount: OrderAmount,
}

#[derive(Serialize)]
struct OrderAmount {
    total: i64, // 分
    currency: &'static str,
}

/// 微信返回的 prepay_id。
#[derive(Deserialize)]
struct CreateOrderResponse {
    prepay_id: Option<String>,
    code: Option<String>,
    message: Option<String>,
}

/// 返回给 Android 端的签名参数（用于拉起微信收银台）。
#[derive(Serialize)]
pub struct AppPayParams {
    pub appid: String,
    pub partnerid: String,
    pub prepayid: String,
    pub package: String,
    pub noncestr: String,
    pub timestamp: String,
    pub sign: String,
}

// ── 对外接口 ──────────────────────────────────────────────────────────────────

/// 创建 App 支付订单，返回 Android 端调起微信所需的签名参数。
///
/// - `out_trade_no`：商户订单号（唯一，建议 `{userId_prefix}_{uuid}`）
/// - `amount_fen`：金额（分）
/// - `description`：商品描述（显示在微信支付页）
pub async fn create_app_order(
    cfg: &WechatPayConfig,
    out_trade_no: &str,
    amount_fen: i64,
    description: &str,
) -> Result<AppPayParams> {
    let url_path = "/v3/pay/transactions/app";
    let body = serde_json::to_string(&CreateAppOrderRequest {
        appid: &cfg.app_id,
        mchid: &cfg.mch_id,
        description,
        out_trade_no,
        notify_url: &cfg.notify_url,
        amount: OrderAmount {
            total: amount_fen,
            currency: "CNY",
        },
    })?;

    let auth = build_auth_header(cfg, "POST", url_path, &body)?;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("https://api.mch.weixin.qq.com{url_path}"))
        .header("Authorization", auth)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| anyhow!("调用微信支付 API 失败: {e}"))?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!("微信支付下单失败 HTTP {status}: {text}"));
    }

    let parsed: CreateOrderResponse =
        serde_json::from_str(&text).map_err(|e| anyhow!("解析微信响应失败: {e}\n原始: {text}"))?;

    if let (Some(code), Some(msg)) = (&parsed.code, &parsed.message) {
        return Err(anyhow!("微信支付错误 {code}: {msg}"));
    }

    let prepay_id = parsed
        .prepay_id
        .ok_or_else(|| anyhow!("微信未返回 prepay_id: {text}"))?;

    // 对 prepayid 重新签名，供 Android 调起支付
    let ts = timestamp_secs();
    let nonce = new_nonce();
    let sign_msg = format!("{}\n{ts}\n{nonce}\n{prepay_id}\n", cfg.app_id);
    let sign = rsa_sign(&cfg.private_key_pem, &sign_msg)?;

    Ok(AppPayParams {
        appid: cfg.app_id.clone(),
        partnerid: cfg.mch_id.clone(),
        prepayid: prepay_id,
        package: "Sign=WXPay".to_string(),
        noncestr: nonce,
        timestamp: ts.to_string(),
        sign,
    })
}

// ── 回调通知结构体 ────────────────────────────────────────────────────────────

/// 微信支付异步通知（POST body）
#[derive(Deserialize)]
pub struct PayNotifyBody {
    pub event_type: String,
    pub resource: NotifyResource,
}

#[derive(Deserialize)]
pub struct NotifyResource {
    pub algorithm: String,
    pub ciphertext: String,
    pub nonce: String,
    pub associated_data: Option<String>,
}

/// 解密后的支付成功通知内容（核心字段）
#[derive(Deserialize)]
pub struct TransactionNotify {
    pub out_trade_no: String,
    pub transaction_id: Option<String>,
    pub trade_state: String,
    pub amount: TransactionAmount,
    pub payer: Option<TransactionPayer>,
}

#[derive(Deserialize)]
pub struct TransactionAmount {
    pub total: i64,
}

#[derive(Deserialize)]
pub struct TransactionPayer {
    pub openid: Option<String>,
}

/// 从回调 body 解密并解析支付通知。
pub fn parse_pay_notify(cfg: &WechatPayConfig, body: &PayNotifyBody) -> Result<TransactionNotify> {
    let aad = body.resource.associated_data.as_deref().unwrap_or("");
    let plaintext = decrypt_notify_resource(
        &cfg.api_v3_key,
        &body.resource.nonce,
        aad,
        &body.resource.ciphertext,
    )?;
    serde_json::from_str(&plaintext)
        .map_err(|e| anyhow!("解析通知内容失败: {e}\n原始: {plaintext}"))
}
