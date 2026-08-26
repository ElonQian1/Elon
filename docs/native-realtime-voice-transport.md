---
capability_id: android_openai_native_realtime_voice_v1
implementation_status: implemented
verification_status: targeted_tests_passed_device_pending
production_default: true
reviewed_at: 2026-08-26
---

# Native realtime voice transport

The production friend-chat surface now exposes two explicit realtime voice transports.

- `android_chatgpt_web_realtime_voice_v1` stays account-bound. It uses the official
  ChatGPT WebView/WebRTC session and writes to the current ChatGPT conversation.
- `android_openai_native_realtime_voice_v1` is a native media path. Android captures
  PCM, sends it through the authenticated 一龙 realtime WebSocket, and plays returned
  PCM through `AudioTrack`. It always creates or continues the local 一龙 AI history;
  it never claims to be the user's ChatGPT web-account conversation.

The friend-chat phone action starts the native transport as a non-blocking floating
voice session. The ChatGPT composer blue voice action continues to start the official
account-bound transport. The ChatGPT tool sheet also exposes `原生实时 AI` so the user
can deliberately start a new local voice conversation.

Both transports reuse the same foreground-service notification and media-focus policy.
The native transport pauses microphone capture without closing its WebSocket, so media
playback and manual pause/resume do not require a fresh handshake. Failure remains
visible in the floating control and can fall back to official ChatGPT voice.

The native transport does not read or export ChatGPT cookies, credentials, request
headers, projects, models, or private conversation content. Server authentication uses
the existing 一龙 app session. Device verification is required before changing this
document's verification status to `device_verified`.
