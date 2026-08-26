const ALLOWED_ORIGIN: &str = "https://chatgpt.com";
pub(super) const ADAPTER_VERSION: u32 = 187;

const WIN_RICH_CONTENT_ADAPTER: &str = include_str!("chatgpt_rich_content_adapter.js");
const WIN_COMMON_RICH_CONTENT_ADAPTER: &str = include_str!("rich_content_dom_adapter.js");
const WIN_CITATION_ADAPTER: &str = include_str!("chatgpt_citation_adapter.js");
const WIN_NEW_CONVERSATION_GUARD: &str = include_str!("chatgpt_win_new_conversation_guard.js");
const WIN_RESPONSE_RESEARCH_CAPTURE: &str = include_str!("win_web_response_research_capture.js");
const WIN_PRIVATE_STREAM_RECOVERY: &str = include_str!("chatgpt_win_private_stream_recovery.js");
const WIN_PRIVATE_FINANCE_PERIODS: &str = include_str!("chatgpt_win_private_finance_periods.js");
const WIN_PRIVATE_SOURCE_GROUPS: &str = include_str!("chatgpt_win_private_source_groups.js");
const WIN_PRIVATE_CONVERSATION_REFRESH: &str =
    include_str!("chatgpt_win_private_conversation_refresh.js");
const WIN_PRIVATE_CONVERSATION_RICH_CACHE: &str =
    include_str!("chatgpt_win_private_conversation_rich_cache.js");
const PRIVATE_FETCH_TAP: &str =
    include_str!("../../../../android/app/src/main/assets/chatgpt_web_private_fetch_tap.js");
const PRIVATE_SOCKET_TAP: &str =
    include_str!("../../../../android/app/src/main/assets/chatgpt_web_private_socket_tap.js");
const PRIVATE_CONVERSATION_DIRECTORY: &str = include_str!(
    "../../../../android/app/src/main/assets/chatgpt_web_private_conversation_directory.js"
);

const ADAPTER_ASSETS: &[(&str, &str)] = &[
    (
        "chatgpt_web_adapter_bootstrap.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_bootstrap.js"),
    ),
    (
        "chatgpt_web_adapter_authentication_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_authentication_policy.js"),
    ),
    (
        "chatgpt_web_private_conversation_directory.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_private_conversation_directory.js"),
    ),
    (
        "chatgpt_web_adapter_project_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_project_policy.js"),
    ),
    (
        "chatgpt_web_adapter_project_hints.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_project_hints.js"),
    ),
    (
        "chatgpt_web_adapter_context_menu_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_context_menu_policy.js"),
    ),
    (
        "chatgpt_web_adapter_conversation_history.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_conversation_history.js"),
    ),
    (
        "chatgpt_web_adapter_conversations.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_conversations.js"),
    ),
    (
        "chatgpt_web_adapter_message_action_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_message_action_policy.js"),
    ),
    (
        "chatgpt_web_adapter_message_portal_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_message_portal_policy.js"),
    ),
    (
        "chatgpt_web_adapter_messages.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_messages.js"),
    ),
    (
        "chatgpt_web_adapter_model_label_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_model_label_policy.js"),
    ),
    (
        "chatgpt_web_adapter_composer_option_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_composer_option_policy.js"),
    ),
    (
        "chatgpt_web_adapter_composer_submenu.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_composer_submenu.js"),
    ),
    (
        "chatgpt_web_adapter_composer_tool_state_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_composer_tool_state_policy.js"),
    ),
    (
        "chatgpt_web_adapter_composer_tool_selection.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_composer_tool_selection.js"),
    ),
    (
        "chatgpt_web_adapter_action_target_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_action_target_policy.js"),
    ),
    (
        "chatgpt_web_adapter_attachment_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_attachment_policy.js"),
    ),
    (
        "chatgpt_web_adapter_dictation_session_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_dictation_session_policy.js"),
    ),
    (
        "chatgpt_web_adapter_composer.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_composer.js"),
    ),
    (
        "chatgpt_web_adapter_navigation_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_navigation_policy.js"),
    ),
    (
        "chatgpt_web_adapter_navigation.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_navigation.js"),
    ),
    (
        "chatgpt_web_adapter_page_semantic_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_page_semantic_policy.js"),
    ),
    (
        "chatgpt_web_adapter_temporary_chat.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_temporary_chat.js"),
    ),
    (
        "chatgpt_web_adapter_form_controls.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_form_controls.js"),
    ),
    (
        "chatgpt_web_adapter_control_ownership_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_control_ownership_policy.js"),
    ),
    (
        "chatgpt_web_adapter_overlay_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_overlay_policy.js"),
    ),
    (
        "chatgpt_web_adapter_form_commands.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_form_commands.js"),
    ),
    (
        "chatgpt_web_adapter_disclosure_controls.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_disclosure_controls.js"),
    ),
    (
        "chatgpt_web_adapter_snapshot_scheduler.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_snapshot_scheduler.js"),
    ),
    (
        "chatgpt_web_adapter_streaming_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_streaming_policy.js"),
    ),
    (
        "chatgpt_web_stream_watchdog_probe.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_stream_watchdog_probe.js"),
    ),
    (
        "chatgpt_web_adapter_skin.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_skin.js"),
    ),
    (
        "chatgpt_web_adapter_realtime_voice_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_realtime_voice_policy.js"),
    ),
    (
        "chatgpt_web_adapter_layout.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_layout.js"),
    ),
    (
        "chatgpt_web_private_research_probe.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_private_research_probe.js"),
    ),
    (
        "chatgpt_web_private_transport_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_private_transport_policy.js"),
    ),
    (
        "chatgpt_web_private_transport.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_private_transport.js"),
    ),
    (
        "chatgpt_web_private_stream_policy.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_private_stream_policy.js"),
    ),
    (
        "chatgpt_web_private_stream_transport.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_private_stream_transport.js"),
    ),
    (
        "chatgpt_web_private_send_observer.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_private_send_observer.js"),
    ),
    (
        "chatgpt_web_adapter.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter.js"),
    ),
];

pub(super) fn initialization_script() -> String {
    let research_capture = WIN_RESPONSE_RESEARCH_CAPTURE.replace("__PROVIDER_ID__", "chatgpt");
    let adapters = ADAPTER_ASSETS
        .iter()
        .map(|(name, source)| {
            let shared = format!(
                "window.__elonChatGptBootstrapStage = '{}';\n{}",
                name, source
            );
            if *name == "chatgpt_web_adapter_messages.js" {
                format!(
                    "window.__elonChatGptBootstrapStage = 'rich_content_dom_adapter.js';\n{}\nwindow.__elonChatGptBootstrapStage = 'chatgpt_rich_content_adapter.js';\n{}\n{}\nwindow.__elonChatGptBootstrapStage = 'chatgpt_citation_adapter.js';\n{}\nwindow.__elonChatGptBootstrapStage = 'chatgpt_win_new_conversation_guard.js';\n{}",
                    WIN_COMMON_RICH_CONTENT_ADAPTER,
                    WIN_RICH_CONTENT_ADAPTER,
                    shared,
                    WIN_CITATION_ADAPTER,
                    WIN_NEW_CONVERSATION_GUARD
                )
            } else if *name == "chatgpt_web_private_stream_policy.js" {
                format!(
                    "{}\nwindow.__elonChatGptBootstrapStage = 'chatgpt_win_private_finance_periods.js';\n{}\nwindow.__elonChatGptBootstrapStage = 'chatgpt_win_private_source_groups.js';\n{}",
                    shared, WIN_PRIVATE_FINANCE_PERIODS, WIN_PRIVATE_SOURCE_GROUPS
                )
            } else if *name == "chatgpt_web_private_transport.js" {
                format!(
                    "window.__elonChatGptBootstrapStage = 'chatgpt_win_private_conversation_rich_cache.js';\n{}\n{}\nwindow.__elonChatGptBootstrapStage = 'chatgpt_win_private_conversation_refresh.js';\n{}",
                    WIN_PRIVATE_CONVERSATION_RICH_CACHE, shared, WIN_PRIVATE_CONVERSATION_REFRESH
                )
            } else if *name == "chatgpt_web_private_stream_transport.js" {
                format!(
                    "{}\nwindow.__elonChatGptBootstrapStage = 'chatgpt_win_private_stream_recovery.js';\n{}",
                    shared, WIN_PRIVATE_STREAM_RECOVERY
                )
            } else {
                shared
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    r#"
(function () {
  'use strict';
  if (location.origin !== '__ALLOWED_ORIGIN__') return;
  // The current Win channel is a pre-launch development cohort. Enable the
  // already-reviewed same-origin read-only research paths so the stable native
  // AST can learn from the official stream before the DOM finishes composing.
  window.__elonChatGptPrivateResearchEnabled = true;
  window.__elonChatGptPrivateStreamObserverEnabled = true;
  window.__elonChatGptPrivateConversationPrefetchEnabled = true;
  window.__elonChatGptBootstrapStage = 'chatgpt_web_private_fetch_tap.js';
  __PRIVATE_FETCH_TAP__
  window.__elonChatGptBootstrapStage = 'chatgpt_web_private_socket_tap.js';
  __PRIVATE_SOCKET_TAP__
  window.__elonChatGptBootstrapStage = 'chatgpt_web_private_conversation_directory.js';
  __PRIVATE_CONVERSATION_DIRECTORY__
  __RESPONSE_RESEARCH_CAPTURE__

  var touchPurposes = new Set([
    'list_model_options', 'list_composer_tools', 'select_model_option', 'select_composer_tool',
    'open_model_submenu', 'open_composer_tools_submenu', 'open_model_selector',
    'open_composer_tools', 'start_dictation', 'cancel_dictation', 'submit_dictation',
    'remove_attachment', 'list_navigation', 'select_navigation', 'dismiss_navigation',
    'invoke_ui_control', 'regenerate_open_menu', 'regenerate_retry'
  ]);

  function dispatchLocalTouch(payload) {
    var envelope;
    try { envelope = JSON.parse(String(payload || '{}')); } catch (_) { return; }
    var event = envelope && envelope.event;
    if (!event || event.type !== 'web_touch_request' || !touchPurposes.has(String(event.purpose || ''))) return;
    var x = Number(event.xRatio); var y = Number(event.yRatio);
    if (!Number.isFinite(x) || !Number.isFinite(y) || x < 0 || x > 1 || y < 0 || y > 1) return;
    window.setTimeout(function () {
      var node = document.elementFromPoint(x * window.innerWidth, y * window.innerHeight);
      if (!(node instanceof HTMLElement) || !node.isConnected) return;
      try { node.focus({ preventScroll: true }); } catch (_) {}
      node.click();
    }, 0);
  }

  function invoke(payload) {
    dispatchLocalTouch(payload);
    var internalInvoke = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke;
    var publicInvoke = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
    var call = internalInvoke || publicInvoke;
    if (typeof call === 'function') {
      Promise.resolve(call('publish_local_ai_web_event', { payload: String(payload || '') })).catch(function () {});
    }
  }

  window.elonChatGptNative = Object.freeze({ postMessage: invoke });
  if (!window.__elonWinChatGptDiagnosticsInstalled) {
    window.__elonWinChatGptDiagnosticsInstalled = true;
    window.addEventListener('error', function (event) {
      invoke(JSON.stringify({
        type: 'browser_diagnostic',
        kind: 'page_error',
        detail: String(event && event.message || 'ChatGPT 页面脚本加载失败。').slice(0, 240),
        url: location.origin + location.pathname
      }));
    });
    window.addEventListener('unhandledrejection', function () {
      invoke(JSON.stringify({
        type: 'browser_diagnostic',
        kind: 'promise_rejection',
        detail: 'ChatGPT 页面尚未完成初始化，可尝试刷新或显示官方页。',
        url: location.origin + location.pathname
      }));
    });
  }

  function documentToken() {
    var words = new Uint32Array(4);
    if (window.crypto && typeof window.crypto.getRandomValues === 'function') {
      window.crypto.getRandomValues(words);
    } else {
      for (var index = 0; index < words.length; index += 1) {
        words[index] = Math.floor(Math.random() * 0xffffffff) >>> 0;
      }
    }
    return 'doc_win_' + Array.from(words, function (word) {
      return word.toString(16).padStart(8, '0');
    }).join('');
  }

  if (!/^doc_[a-z0-9_]{3,80}$/.test(String(window.__elonChatGptDocumentToken || ''))) {
    window.__elonChatGptDocumentToken = documentToken();
  }
  function installAdapter() {
    try {
      window.__elonChatGptAdapterTargetVersion = __ADAPTER_VERSION__;
      __ADAPTER_ASSETS__
      window.__elonChatGptBootstrapStage = 'bridge_check';
      if (!window.__elonChatGptBridge || typeof window.__elonChatGptBridge.command !== 'function') {
        throw new Error('bridge_missing');
      }
      window.__elonChatGptBootstrapStage = 'ready';
    } catch (error) {
      var errorName = String(error && error.name || 'Error').replace(/[^A-Za-z0-9_]/g, '').slice(0, 40);
      var errorStage = String(window.__elonChatGptBootstrapStage || 'unknown')
        .replace(/[^A-Za-z0-9_.-]/g, '').slice(0, 80);
      invoke(JSON.stringify({
        type: 'browser_diagnostic',
        kind: 'adapter_bootstrap_failed',
        detail: 'ChatGPT 语义桥初始化失败（' + (errorName || 'Error') +
          '，阶段：' + (errorStage || 'unknown') + '）。',
        url: location.origin + location.pathname
      }));
    }
  }

  if (document.readyState === 'loading') {
    window.addEventListener('DOMContentLoaded', installAdapter, { once: true });
  } else {
    installAdapter();
  }
})();
"#
    .replace("__ALLOWED_ORIGIN__", ALLOWED_ORIGIN)
    .replace("__ADAPTER_VERSION__", &ADAPTER_VERSION.to_string())
    .replace("__PRIVATE_FETCH_TAP__", PRIVATE_FETCH_TAP)
    .replace("__PRIVATE_SOCKET_TAP__", PRIVATE_SOCKET_TAP)
    .replace(
        "__PRIVATE_CONVERSATION_DIRECTORY__",
        PRIVATE_CONVERSATION_DIRECTORY,
    )
    .replace("__RESPONSE_RESEARCH_CAPTURE__", &research_capture)
    .replace("__ADAPTER_ASSETS__", &adapters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn win_bootstrap_tracks_the_complete_android_adapter_bundle() {
        let script = initialization_script();
        let android_page_adapter = include_str!(
            "../../../../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt"
        );
        let android_assets = android_page_adapter
            .split("private val ADAPTER_ASSETS = listOf(")
            .nth(1)
            .and_then(|tail| tail.split("\n        )").next())
            .expect("Android ChatGPT adapter asset list should remain readable")
            .lines()
            .filter_map(|line| {
                let value = line.trim().trim_end_matches(',');
                value
                    .strip_prefix('"')
                    .and_then(|value| value.strip_suffix('"'))
            })
            .collect::<Vec<_>>();
        let android_version = android_page_adapter
            .split("internal const val ADAPTER_VERSION = ")
            .nth(1)
            .and_then(|tail| tail.lines().next())
            .and_then(|value| value.trim().parse::<u32>().ok())
            .expect("Android ChatGPT adapter version should remain readable");
        let win_assets = ADAPTER_ASSETS
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>();

        assert_eq!(win_assets, android_assets);
        assert_eq!(ADAPTER_VERSION, android_version);
        assert!(script.contains(&format!(
            "__elonChatGptAdapterTargetVersion = {ADAPTER_VERSION}"
        )));
        assert!(script.contains("__elonChatGptDocumentToken"));
        assert!(script.contains("__elonChatGptSnapshotScheduler"));
        assert!(script.contains("__elonChatGptLayout"));
        assert!(script.contains("window.__elonChatGptBridge"));
        assert!(script.contains("DOMContentLoaded"));
        assert!(script.contains("adapter_bootstrap_failed"));
        assert!(script.contains("dispatchLocalTouch"));
        assert!(script.contains("document.elementFromPoint"));
        assert!(script.contains("阶段："));
        assert!(script.contains("chatgpt_web_adapter_composer_submenu.js"));
        assert!(script.contains("chatgpt_web_adapter_message_portal_policy.js"));
        assert!(script.contains("chatgpt_web_adapter_project_hints.js"));
        assert!(script.contains("chatgpt_citation_adapter.js"));
        assert!(script.contains("__elonChatGptCitationAdapter"));
        assert!(script.contains("__elonWinChatGptNewConversationGuard"));
        assert!(script.contains("官网未离开上一会话，已转入安全恢复。"));
        assert!(script.contains("publish_local_ai_web_research_capture"));
        assert!(script.contains("conversation_stream"));
        assert!(script.contains("__elonChatGptPrivateStreamObserverEnabled = true"));
        assert!(script.contains("__elonChatGptPrivateFetchTap"));
        assert!(script.contains("__elonChatGptPrivateSocketTap"));
        assert!(
            script.find("chatgpt_web_private_fetch_tap.js").unwrap()
                < script
                    .find("chatgpt_web_private_stream_transport.js")
                    .unwrap()
        );
        assert!(script.contains("__elonChatGptPrivateConversationDirectory"));
        assert!(script.contains("chatgpt_web_private_stream_transport.js"));
        assert!(script.contains("chatgpt_win_private_finance_periods.js"));
        assert!(script.contains("chatgpt_win_private_source_groups.js"));
        assert!(script.contains("__elonWinChatGptPrivateFinancePeriods"));
        assert!(
            script.find("chatgpt_web_private_stream_policy.js").unwrap()
                < script
                    .find("chatgpt_win_private_finance_periods.js")
                    .unwrap()
        );
        assert!(
            script
                .find("chatgpt_win_private_finance_periods.js")
                .unwrap()
                < script
                    .find("chatgpt_web_private_stream_transport.js")
                    .unwrap()
        );
        assert!(
            script.find("chatgpt_win_private_source_groups.js").unwrap()
                < script
                    .find("chatgpt_web_private_stream_transport.js")
                    .unwrap()
        );
        assert!(script.contains("privateStreamingSnapshotMode"));
        assert!(script.contains("privateStreamWatchdogMs"));
        assert!(script.contains("privateStreamObserved"));
        assert!(script.contains("chatgpt_win_private_conversation_refresh.js"));
        assert!(script.contains("__elonWinConversationRefreshWrapped"));
        assert!(
            script.find("chatgpt_web_private_transport.js").unwrap()
                < script
                    .find("chatgpt_win_private_conversation_refresh.js")
                    .unwrap()
        );
        assert!(
            script
                .find("chatgpt_win_private_conversation_refresh.js")
                .unwrap()
                < script.find("chatgpt_web_adapter.js").unwrap()
        );
        assert!(script.contains("chatgpt_win_private_stream_recovery.js"));
        assert!(script.contains("__elonWinChatGptPrivateStreamRecovery"));
        assert!(
            script
                .find("chatgpt_web_private_stream_transport.js")
                .unwrap()
                < script
                    .find("chatgpt_win_private_stream_recovery.js")
                    .unwrap()
        );
        assert!(
            script
                .find("chatgpt_win_private_stream_recovery.js")
                .unwrap()
                < script.find("chatgpt_web_adapter.js").unwrap()
        );
        let dom_ready = script
            .find("DOMContentLoaded")
            .expect("Win bootstrap should retain its DOM-ready semantic install");
        assert!(script.find("__elonChatGptPrivateSocketTap").unwrap() < dom_ready);
        assert!(
            script
                .find("__elonChatGptPrivateConversationDirectory")
                .unwrap()
                < dom_ready
        );
    }
}
