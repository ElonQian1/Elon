use super::*;
#[test]
fn binance_app_shares_preserve_url_and_only_allow_exact_host() {
    let url = policy::public_url("https://app.binance.com/uni-qr/cpos/123456?r=synthetic&l=zh-CN")
        .unwrap();
    assert_eq!(policy::site(&url), "币安广场");
    assert!(policy::fetchable(&url));
    assert_eq!(Preview::fallback(&url).url, url.as_str());
    let spoof =
        policy::public_url("https://app.binance.com.evil.example/uni-qr/cpos/123456").unwrap();
    assert!(!policy::fetchable(&spoof));
}
#[test]
fn social_link_preview_official_players_preserve_time_and_page() {
    let url = policy::public_url(
        "https://www.bilibili.com/video/BV1enYL6SEtU/?share_source=copy_web&t=2&p=3",
    )
    .unwrap();
    let player = policy::embed(&url).unwrap();
    assert_eq!(
        player.url,
        "https://player.bilibili.com/player.html?bvid=BV1enYL6SEtU&autoplay=0&poster=1&t=2&p=3"
    );
    assert!(
        policy::embed(&policy::public_url("https://x.com/i/article/123456789").unwrap()).is_none()
    );
    assert_eq!(
        policy::embed(
            &policy::public_url("https://twitter.com/Interior/status/463440424141459456?s=20")
                .unwrap()
        )
        .unwrap()
        .kind,
        "x"
    );
}
#[test]
fn social_link_preview_does_not_trust_suffix_lookalikes_or_credentials() {
    for value in [
        "http://www.bilibili.com/video/BV1enYL6SEtU",
        "https://user:pass@x.com/a",
        "https://x.com:8443/a",
        "file:///tmp/foo",
    ] {
        assert!(policy::public_url(value).is_none());
    }
    for value in [
        "https://x.com.evil.example/a/status/123456",
        "https://127.0.0.1/a",
        "https://evil.example/?url=https://x.com",
    ] {
        assert!(!policy::fetchable(&policy::public_url(value).unwrap()));
    }
    assert!(policy::image_url(
        "https://hdslb.com.evil.example/a.jpg",
        &policy::public_url("https://www.bilibili.com").unwrap()
    )
    .is_none());
}
#[test]
fn social_link_preview_metadata_is_text_and_skips_scripts() {
    let url = policy::public_url("https://mp.weixin.qq.com/s/test").unwrap();
    let html = r#"<head><script>let fake='<meta property="og:title" content="evil">';</script><!-- <meta property="og:title" content="comment"> --><title>fallback</title><meta content='A &amp; B &quot;quoted&quot;' property='og:title'><meta property="og:image" content="https://mmbiz.qpic.cn/cover.jpg?a=1&amp;b=2"></head><body><meta property="og:title" content="body"></body>"#;
    let result = metadata::parse(html, &url);
    assert_eq!(result.title, "A & B \"quoted\"");
    assert_eq!(
        result.image.as_deref(),
        Some("https://mmbiz.qpic.cn/cover.jpg?a=1&b=2")
    );
    assert_eq!(
        metadata::parse("<meta disabled property=og:title content='x > y'>", &url).title,
        "x > y"
    );
}
#[test]
fn social_link_preview_shares_keep_required_parameters_and_cache_is_disposable() {
    let url = policy::public_url(
        "https://www.xiaohongshu.com/discovery/item/abc?xsec_token=synthetic&xsec_source=pc_share",
    )
    .unwrap();
    let fallback = Preview::fallback(&url);
    assert!(fallback.url.contains("xsec_token=synthetic"));
    assert_eq!(ttl(&fallback), Duration::from_secs(30));
    assert!(fallback.embed.is_none());
}

#[test]
fn social_link_preview_skips_generic_logo_and_checks_upgraded_cover() {
    let url = policy::public_url("https://www.xiaohongshu.com/discovery/item/example").unwrap();
    let meta = metadata::parse(
        r#"<meta property="og:image" content="https://picasso-static.xiaohongshu.com/fe-platform/icon.png"><meta property="og:image" content="http://sns-webpic-qc.xhscdn.com/cover.jpg">"#,
        &url,
    );
    assert_eq!(
        meta.image.as_deref(),
        Some("https://sns-webpic-qc.xhscdn.com/cover.jpg")
    );
    assert!(meta.image_needs_check);
}

#[test]
fn social_link_preview_reads_description_and_site_agents() {
    let url = policy::public_url("https://mp.weixin.qq.com/s/test").unwrap();
    let meta = metadata::parse(
        r#"<meta name="description" content="generic"><meta property="og:description" content="  正文&amp;摘要  "><meta name="twitter:description" content="ignored">"#,
        &url,
    );
    assert_eq!(meta.description, "正文&摘要");
    assert!(policy::user_agent(&url).starts_with("Mozilla/5.0 (Windows"));
    let binance = policy::public_url("https://www.binance.com/en/square/post/1234567").unwrap();
    assert!(policy::user_agent(&binance).contains("Discordbot"));
}

#[test]
fn social_link_report_requires_same_identity_and_allowed_cover() {
    let original = policy::public_url("https://app.binance.com/uni-qr/cpos/123456?r=abc").unwrap();
    let read = |url: &str, title: &str, image: Option<&str>| report::Read {
        schema: 1,
        original: original.to_string(),
        url: url.into(),
        article: true,
        title: title.into(),
        author: " Author ".into(),
        description: "d".into(),
        image: image.map(Into::into),
    };
    let ok = report::validate(
        &original,
        &read(
            "https://www.binance.com/zh-CN/square/post/123456",
            "Real post",
            Some("https://public.bnbstatic.com/image/cms/content.png"),
        ),
    )
    .unwrap();
    assert_eq!(
        (ok.title.as_str(), ok.author.as_str()),
        ("Real post", "Author")
    );
    assert!(ok.image.is_some());
    assert!(report::validate(
        &original,
        &read(
            "https://www.binance.com/en/square/post/999999",
            "Other",
            None
        )
    )
    .is_err());
    assert!(report::validate(
        &original,
        &read(
            "https://www.binance.com/en/square/post/123456",
            "Sign in to Binance",
            None
        )
    )
    .is_err());
    let avatar = report::validate(
        &original,
        &read(
            "https://www.binance.com/en/square/post/123456",
            "Post",
            Some("https://public.bnbstatic.com/static/avatar/1.png"),
        ),
    )
    .unwrap();
    assert!(avatar.image.is_none());
    let x = policy::public_url("https://x.com/author/status/463440424141459456").unwrap();
    assert_eq!(
        report::identity(&x).as_deref(),
        Some("x-post:463440424141459456")
    );
    assert_eq!(
        report::identity(&policy::public_url("https://mp.weixin.qq.com/s?__biz=1&mid=2").unwrap())
            .as_deref(),
        Some("wechat:/s?__biz=1&mid=2")
    );
    assert!(report::identity(&policy::public_url("https://t.co/abc").unwrap()).is_none());
}

#[tokio::test]
async fn social_link_report_does_not_override_server_result() {
    let url = policy::public_url("https://mp.weixin.qq.com/s/report-test").unwrap();
    let key: [u8; 32] = Sha256::digest(url.as_str()).into();
    let mut ready = Preview::fallback(&url);
    ready.title = "server title".into();
    ready.status = "ready";
    store(key, ready);
    let read = report::Read {
        schema: 1,
        original: url.to_string(),
        url: url.to_string(),
        article: true,
        title: "member title".into(),
        author: String::new(),
        description: String::new(),
        image: None,
    };
    let kept = report("user-a", url.clone(), &read).await.unwrap();
    assert_eq!(
        (kept.title.as_str(), kept.source),
        ("server title", "server")
    );
    CACHE.lock().unwrap().remove(&key);
    let member = report("user-a", url.clone(), &read).await.unwrap();
    assert_eq!(
        (member.title.as_str(), member.source, member.status),
        ("member title", "member", "ready")
    );
    assert_eq!(cached(url).await.title, "member title");
    CACHE.lock().unwrap().remove(&key);
}

#[test]
fn social_link_cover_thumbnail_is_small_inline_jpeg() {
    let mut png = Vec::new();
    image::RgbImage::from_pixel(900, 600, image::Rgb([200, 30, 30]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let data = cover::thumbnail(&png).unwrap();
    assert!(data.starts_with("data:image/jpeg;base64,"));
    assert!(data.len() < cover::MAX_DATA_URL);
    assert!(cover::thumbnail(b"not an image").is_err());
}

#[tokio::test]
async fn social_link_preview_reuses_completed_cache_without_network() {
    let url =
        policy::public_url("https://www.bilibili.com/video/BV1enYL6SEtU/?t=2&test=cache").unwrap();
    let mut expected = Preview::fallback(&url);
    expected.title = "cached title".into();
    expected.status = "ready";
    let cell = Arc::new(OnceCell::new());
    cell.set((Instant::now(), expected)).unwrap();
    let key: [u8; 32] = Sha256::digest(url.as_str()).into();
    CACHE.lock().unwrap().insert(key, cell);
    let (first, second) = tokio::join!(cached(url.clone()), cached(url));
    assert_eq!(first.title, "cached title");
    assert_eq!(second.title, "cached title");
    CACHE.lock().unwrap().remove(&key);
}

#[tokio::test]
#[ignore = "read-only official public network smoke; availability depends on provider"]
async fn social_link_preview_live_public_sources() {
    let samples = [
        "https://mp.weixin.qq.com/s/7Z9FCc1GVf6PND1t_h9E9g",
        "https://www.bilibili.com/video/BV1enYL6SEtU/?t=2",
        "https://v.douyin.com/_XMEsxVKKOY/",
        "https://www.xiaohongshu.com/discovery/item/6a6e8951000000002402c81f",
        "https://www.binance.com/en/square/post/307056334723618",
        "https://x.com/Interior/status/463440424141459456",
    ];
    for sample in samples {
        let url = public_url(sample).unwrap();
        let direct = tokio::time::timeout(Duration::from_secs(12), fetch::resolve(url.clone()))
            .await
            .map_err(|_| "timeout".to_string())
            .and_then(|r| r.map_err(|e| e.to_string()));
        let result = tokio::time::timeout(Duration::from_secs(12), preview(url))
            .await
            .unwrap();
        assert_eq!(result.url, sample);
        println!(
            "provider={} metadata={} cover={} inline_cover={} embed={} title_len={} description_len={} direct_error={:?}",
            result.site,
            result.status,
            result.image.is_some(),
            result.cover_data_url.is_some(),
            result.embed.is_some(),
            result.title.chars().count(),
            result.description.chars().count(),
            direct.err()
        );
    }
}
