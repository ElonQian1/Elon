---
capability_id: android_openai_native_realtime_voice_v1
implementation_status: implemented
verification_status: targeted_tests_passed_device_pending
production_default: false
runtime_enabled: false
user_visible: false
reviewed_at: 2026-08-27
---

# Native realtime voice transport

The production friend-chat surface has one consumer-default realtime voice architecture.

- `android_chatgpt_web_realtime_voice_v1` combines native UI, the persistent background
  ChatGPT WebView identity/session, and official page-created WebRTC. It writes to the
  current ChatGPT conversation and is the only consumer-visible default.
- `android_openai_native_realtime_voice_v1` is a native media path. Android captures
  PCM, sends it through the authenticated 一龙 realtime WebSocket, and plays returned
  PCM through `AudioTrack`. It always creates or continues the local 一龙 AI history;
  it never claims to be the user's ChatGPT web-account conversation. Its implementation
  remains for diagnostics, but it is disabled and has no ordinary-user entry.

The friend-chat phone action, blue composer voice action, and semantic/MCP voice command
all start the official account-bound transport. The ChatGPT tool sheet does not expose
the server API experiment.

Both transports reuse the same foreground-service notification and media-focus policy.
The native transport pauses microphone capture without closing its WebSocket, so media
playback and manual pause/resume do not require a fresh handshake. Failure remains
visible in the floating control and can fall back to official ChatGPT voice.

The native transport does not read or export ChatGPT cookies, credentials, request
headers, projects, models, or private conversation content. Server authentication uses
the existing 一龙 app session. Device verification is required before changing this
document's verification status to `device_verified`.
