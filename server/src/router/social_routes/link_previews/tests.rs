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
        let result = tokio::time::timeout(Duration::from_secs(12), preview(url))
            .await
            .unwrap();
        assert_eq!(result.url, sample);
        println!(
            "provider={} metadata={} cover={} embed={}",
            result.site,
            result.status,
            result.image.is_some(),
            result.embed.is_some()
        );
    }
}
