use super::*;

pub(super) async fn open(
    app: AppHandle,
    webview: Webview,
    runtime: State<'_, LocalAiBrowserRuntime>,
    provider: &'static ProviderDefinition,
    owner_key: String,
    show_window: bool,
    task_id: Option<&str>,
) -> Result<LocalAiWebSession, String> {
    let owner_fingerprint = resolve_owner_fingerprint(&app, provider, &owner_key)?;
    if task_id.is_some() {
        crate::group_ai_worker::ensure_caller(&webview)?;
    } else {
        ensure_session_webview(&webview, provider, &owner_fingerprint)?;
    }
    let _creation_guard = lock_webview_creation();
    let window_label = match task_id {
        Some(id) => group_session::label(provider, &owner_fingerprint, id)?,
        None => window_label(provider, &owner_fingerprint),
    };
    if task_id.is_some() {
        let prefix = format!(
            "{}-group-",
            super::window_label(provider, &owner_fingerprint)
        );
        if runtime.snapshot(&window_label).is_none() && runtime.group_session_count(&prefix) >= 2 {
            return Err("已有群聊 AI 正在运行，请先完成或关闭已有任务。".into());
        }
        runtime.ensure_session(
            &window_label,
            provider.id,
            initial_renderer_status(provider),
        );
    } else {
        ensure_runtime_session(
            &app,
            runtime.inner(),
            provider,
            &owner_fingerprint,
            &window_label,
        )?;
    }

    if let Some(page) = app.get_webview(&window_label) {
        if task_id.is_some() {
            // A login redirect may have left the task document on the home page.
            // Preparing a retry resets only this isolated document, never personal chat.
            let url = if provider.id == "chatgpt" {
                "https://chatgpt.com/?temporary-chat=true"
            } else {
                provider.start_url
            };
            runtime.mark_opening(&window_label, show_window);
            page.navigate(Url::parse(url).map_err(display_error)?)
                .map_err(display_error)?;
        }
        if show_window {
            embedded_view::restore_popout(&app, &window_label)?;
            runtime.mark_window_visible(&window_label, true);
        }
        runtime.mark_window_status(&window_label, "ready");
        request_adapter_snapshot(provider, &page);
        return Ok(session_response(
            provider,
            window_label,
            if show_window { "focused" } else { "background" },
        ));
    }

    let cached_url = if task_id.is_none() {
        runtime.cached_restorable_url(&window_label)
    } else {
        None
    };
    runtime.mark_opening(&window_label, show_window);
    let start_url = if task_id.is_some() && provider.id == "chatgpt" {
        Url::parse("https://chatgpt.com/?temporary-chat=true").map_err(display_error)?
    } else {
        restorable_start_url(provider, cached_url.as_deref())?
    };
    let bootstrap_url = "about:blank"
        .parse()
        .map_err(|error| format!("WebView2 启动页无效：{error}"))?;
    let profile_directory = profile_directory(&app, provider, &owner_fingerprint)?;
    fs::create_dir_all(&profile_directory)
        .map_err(|error| format!("无法创建本地 AI 浏览器 Profile：{error}"))?;

    let navigation_provider = *provider;
    let navigation_state = runtime.inner().clone();
    let navigation_label = window_label.clone();
    let popup_provider = *provider;
    let popup_state = runtime.inner().clone();
    let popup_label = window_label.clone();
    let page_state = runtime.inner().clone();
    let page_label = window_label.clone();
    let page_provider = *provider;
    let page_app = app.clone();
    let window_state = runtime.inner().clone();
    let window_state_label = window_label.clone();

    let popout = if let Some(window) = app.get_window(&window_label) {
        window
    } else {
        WindowBuilder::new(&app, &window_label)
            .title(format!("{} · 一龙本地会话", provider.display_name))
            .inner_size(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT)
            .min_inner_size(900.0, 620.0)
            .center()
            .visible(false)
            .build()
            .map_err(|error| {
                runtime.record_error(
                    &window_label,
                    format!("无法创建 {} 弹出宿主：{error}", provider.display_name),
                );
                display_error(error)
            })?
    };
    let mut builder = WebviewBuilder::new(&window_label, WebviewUrl::External(bootstrap_url))
        .data_directory(profile_directory)
        .incognito(false)
        .enable_clipboard_access();
    if let Some(adapter) = provider.adapter {
        builder = builder.initialization_script(if task_id.is_some() && provider.id == "chatgpt" {
            group_text_bootstrap::initialization_script()
        } else {
            adapter.initialization_script()
        });
    }
    builder = builder
        .on_navigation(move |url| {
            let allowed = allows_navigation(&navigation_provider, url);
            let blocked_message = navigation_block_message(&navigation_provider, url);
            let safe_url = safe_log_url(url);
            navigation_state.mark_navigation(
                &navigation_label,
                url,
                allowed,
                blocked_message.as_deref(),
            );
            println!(
                "[elon-desktop][local-ai] {} 导航 allowed={} -> {}",
                navigation_provider.id, allowed, safe_url
            );
            if !allowed {
                eprintln!(
                    "[elon-desktop][local-ai] 已阻止 {} 导航到 {}",
                    navigation_provider.id, safe_url
                );
            }
            allowed
        })
        .on_new_window(move |url, _features| {
            let allowed = allows_navigation(&popup_provider, &url);
            let blocked_message = navigation_block_message(&popup_provider, &url);
            popup_state.mark_navigation(&popup_label, &url, allowed, blocked_message.as_deref());
            if allowed {
                NewWindowResponse::Allow
            } else {
                NewWindowResponse::Deny
            }
        })
        .on_page_load(move |page, payload| {
            let safe_url = safe_log_url(payload.url());
            println!(
                "[elon-desktop][local-ai] 页面事件 {:?} -> {}",
                payload.event(),
                safe_url
            );
            match payload.event() {
                PageLoadEvent::Started => {
                    page_state.mark_navigation(&page_label, payload.url(), true, None);
                    // WebView2 can momentarily restore a child WebView to its creation
                    // bounds when a provider navigates from the new-chat page to the
                    // freshly created conversation. Waiting for `Finished` leaves the
                    // official page covering the native chat for the whole response.
                    // Re-park at navigation start as well as finish whenever React has
                    // not explicitly granted the official surface foreground ownership.
                    let _ = embedded_view::park_if_background(&page_app, &page_state, &page_label);
                }
                PageLoadEvent::Finished => {
                    page_state.mark_page_finished(&page_label, payload.url());
                    reconnect_adapter(&page_provider, &page);
                    let _ = embedded_view::park_if_background(&page_app, &page_state, &page_label);
                }
            }
        });
    let main_window = app
        .get_window(MAIN_WEBVIEW_LABEL)
        .ok_or_else(|| "一龙主窗口不可用。".to_string())?;
    let page = if !show_window {
        main_window.add_child(
            builder,
            LogicalPosition::new(0.0, 0.0),
            LogicalSize::new(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT),
        )
    } else {
        let size = popout.inner_size().map_err(display_error)?;
        popout.add_child(builder, LogicalPosition::new(0.0, 0.0), size)
    }
    .map_err(|error| {
        runtime.record_error(
            &window_label,
            format!("无法创建 {} WebView2：{error}", provider.display_name),
        );
        display_error(error)
    })?;

    let destroyed_app = app.clone();
    popout.on_window_event(move |event| {
        let resized = match event {
            WindowEvent::Resized(size) => Some(*size),
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => Some(*new_inner_size),
            _ => None,
        };
        if let Some(size) = resized {
            if let Some(page) = destroyed_app.get_webview(&window_state_label) {
                let _ = page.set_position(PhysicalPosition::new(0, 0));
                let _ = page.set_size(size);
            }
        } else if matches!(event, WindowEvent::Destroyed)
            && destroyed_app.get_webview(&window_state_label).is_none()
        {
            window_state.mark_window_status(&window_state_label, "closed");
        }
    });
    if show_window {
        page.show().map_err(display_error)?;
        restore_window(&popout)?;
        page.set_focus().map_err(display_error)?;
    } else {
        embedded_view::hide(&app, &window_label)?;
    }
    runtime.mark_window_visible(&window_label, show_window);
    page.navigate(start_url).map_err(|error| {
        runtime.record_error(
            &window_label,
            format!("{} 首次导航失败：{error}", provider.display_name),
        );
        display_error(error)
    })?;
    Ok(session_response(
        provider,
        window_label,
        if show_window { "created" } else { "background" },
    ))
}
