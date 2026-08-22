use tauri::Webview;

use super::{display_error, LocalAiWebSessionState};

pub(super) fn set_content_surface_mode(webview: &Webview, enabled: bool) -> Result<(), String> {
    let script = if enabled {
        answer_surface_script()
    } else {
        restore_surface_script()
    };
    webview.eval(script).map_err(display_error)
}

pub(super) fn answer_surface_ready(state: &LocalAiWebSessionState) -> bool {
    if state.loading
        || state.semantic_cache_status != "live"
        || state.window_status == "blocked"
        || state.window_status == "error"
    {
        return false;
    }
    let Some(event) = state.semantic_event.as_ref() else {
        return false;
    };
    if event
        .get("streaming")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return false;
    }
    semantic_event_has_completed_assistant(event)
}

fn semantic_event_has_completed_assistant(event: &serde_json::Value) -> bool {
    event
        .get("messages")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|messages| {
            messages.iter().any(|message| {
                message.get("role").and_then(serde_json::Value::as_str) == Some("assistant")
                    && message.get("state").and_then(serde_json::Value::as_str) != Some("streaming")
            })
        })
}

fn answer_surface_script() -> &'static str {
    r#"
(function () {
  'use strict';
  var styleId = 'elon-official-answer-surface-style';
  var backdropId = 'elon-official-answer-surface-backdrop';
  var providerAttribute = 'data-elon-official-answer-provider';
  document.documentElement.removeAttribute('data-elon-official-answer-surface');
  document.documentElement.removeAttribute(providerAttribute);
  document.querySelectorAll('[data-elon-official-answer-root]').forEach(function (node) {
    node.removeAttribute('data-elon-official-answer-root');
  });
  var oldStyle = document.getElementById(styleId);
  if (oldStyle) oldStyle.remove();
  var oldBackdrop = document.getElementById(backdropId);
  if (oldBackdrop) oldBackdrop.remove();

  var root = document.querySelector('main, [role="main"]');
  if (!(root instanceof HTMLElement)) return;

  var provider = /(^|\.)chatgpt\.com$/i.test(window.location.hostname)
    ? 'chatgpt'
    : 'google-ai-mode';

  var style = document.createElement('style');
  style.id = styleId;
  style.textContent = [
    'html[data-elon-official-answer-surface], html[data-elon-official-answer-surface] body { overflow: hidden !important; }',
    'html[data-elon-official-answer-provider="google-ai-mode"], html[data-elon-official-answer-provider="google-ai-mode"] body { background: #202124 !important; }',
    'html[data-elon-official-answer-provider="chatgpt"], html[data-elon-official-answer-provider="chatgpt"] body { background: var(--main-surface-primary, #212121) !important; }',
    '#elon-official-answer-surface-backdrop { position: fixed !important; inset: 0 !important; z-index: 2147483000 !important; background: #202124 !important; }',
    'html[data-elon-official-answer-provider="chatgpt"] #elon-official-answer-surface-backdrop { background: var(--main-surface-primary, #212121) !important; }',
    '[data-elon-official-answer-root] { position: fixed !important; inset: 0 !important; z-index: 2147483001 !important; box-sizing: border-box !important; width: 100vw !important; max-width: none !important; height: 100vh !important; max-height: none !important; margin: 0 !important; overflow: auto !important; overscroll-behavior: contain !important; border-radius: 0 !important; }',
    'html[data-elon-official-answer-provider="google-ai-mode"] [data-elon-official-answer-root] { background: #202124 !important; }',
    'html[data-elon-official-answer-provider="chatgpt"] [data-elon-official-answer-root] { background: var(--main-surface-primary, #212121) !important; scrollbar-gutter: stable; }',
    'html[data-elon-official-answer-provider="chatgpt"] [data-elon-official-answer-root] [data-message-author-role] { box-sizing: border-box !important; width: min(100%, 48rem) !important; max-width: 48rem !important; margin-inline: auto !important; }',
    '[data-elon-official-answer-root] form:has(textarea), [data-elon-official-answer-root] form:has([contenteditable]) { opacity: 0 !important; pointer-events: none !important; }',
    '[data-elon-official-answer-root] textarea, [data-elon-official-answer-root] [contenteditable="true"], [data-elon-official-answer-root] [contenteditable="plaintext-only"] { opacity: 0 !important; pointer-events: none !important; }'
  ].join('\n');
  document.head.appendChild(style);

  var backdrop = document.createElement('div');
  backdrop.id = backdropId;
  backdrop.setAttribute('aria-hidden', 'true');
  document.body.appendChild(backdrop);
  root.setAttribute('data-elon-official-answer-root', 'true');
  document.documentElement.setAttribute(providerAttribute, provider);
  document.documentElement.setAttribute('data-elon-official-answer-surface', 'true');

  window.requestAnimationFrame(function () {
    var candidates = Array.from(root.querySelectorAll(
      '[data-testid^="conversation-turn-"][data-message-author-role="assistant"], [data-message-author-role="assistant"], [data-sfc-cp][data-hveid], article, [role="article"]'
    ));
    var target = candidates.reverse().find(function (node) {
      var rect = node.getBoundingClientRect();
      return rect.width > 240 && rect.height > 40;
    });
    if (target && typeof target.scrollIntoView === 'function') {
      target.scrollIntoView({ block: 'center', inline: 'nearest' });
    }
  });
})();
"#
}

fn restore_surface_script() -> &'static str {
    r#"
(function () {
  'use strict';
  document.documentElement.removeAttribute('data-elon-official-answer-surface');
  document.documentElement.removeAttribute('data-elon-official-answer-provider');
  document.querySelectorAll('[data-elon-official-answer-root]').forEach(function (node) {
    node.removeAttribute('data-elon-official-answer-root');
  });
  var style = document.getElementById('elon-official-answer-surface-style');
  if (style) style.remove();
  var backdrop = document.getElementById('elon-official-answer-surface-backdrop');
  if (backdrop) backdrop.remove();
})();
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn answer_surface_requires_a_completed_assistant_message() {
        assert!(semantic_event_has_completed_assistant(&json!({
            "messages": [
                {"role":"user","state":"completed"},
                {"role":"assistant","state":"completed"}
            ]
        })));
        assert!(!semantic_event_has_completed_assistant(&json!({
            "messages": [{"role":"assistant","state":"streaming"}]
        })));
        assert!(!semantic_event_has_completed_assistant(&json!({
            "messages": [{"role":"user","state":"completed"}]
        })));
    }

    #[test]
    fn answer_surface_script_is_pc_scoped_and_reversible() {
        let apply = answer_surface_script();
        let restore = restore_surface_script();
        assert!(apply.contains("main, [role=\"main\"]"));
        assert!(apply.contains("data-elon-official-answer-root"));
        assert!(apply.contains("data-elon-official-answer-provider"));
        assert!(apply.contains(r"chatgpt\.com"));
        assert!(apply.contains("max-width: 48rem"));
        assert!(apply.contains("form:has(textarea)"));
        assert!(restore.contains("style.remove()"));
        assert!(restore.contains("backdrop.remove()"));
    }
}
