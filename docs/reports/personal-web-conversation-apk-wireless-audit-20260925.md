---
version_status: current
reviewed_at: 2026-09-25
---

# APK Wireless Connection and Private Reader Audit

Baseline: `598699d4ce532e5c6b33aa5453c090ce808d4050`. This follows the
[dual-client check](personal-web-conversation-dual-client-check-20260925.md).
No credentials, account identities, conversation bodies, phone serials or LAN
addresses are retained here.

## Wireless and Installation Evidence

- The user connected the registered Xiaomi 23116PN5BC by authorized USB and
  explicitly authorized future autonomous wireless ADB reconnection.
- The handset's WLAN address matched the main project's registered endpoint.
  TCP ADB was enabled through that verified USB transport. Wireless connection,
  hardware identity, package inspection and disconnect/reconnect all passed.
- USB and Wi-Fi were verified as the same handset. This is USB-bootstrapped TCP
  ADB, not acceptance of Android's paired Wireless debugging mode or persistence
  across reboot/network changes.
- The installed APK was 1810. The official 1811 artifact was downloaded,
  SHA-256 verified and installed over wireless using `adb install -r`.
  Readback confirmed `1.1.1811` / `1811` with existing application data retained.
- Artifact source: `19fe4eb900008f5e3e617ffb1e99f1e31f8a7a1c`;
  SHA-256: `f7c641177229012e96de79f908b2b8e736ace66df07b7be57a5c3fdc308b2036`.
- The existing APK MCP bootstrap and `phone_status` succeeded over a
  handset-bound ADB forward. The local reader configuration now includes the
  loopback APK endpoint and retains its original one-conversation grant.

## Confirmed Adapter Error and Scoped Repair

The first real read failed before opening ChatGPT. Native `ui_control` returned
`isError=true` with structured `chatgpt_web_chat_inactive`. The generic MCP
decoder threw `mcp_call_failed` before the APK adapter could execute its existing
open-once recovery. The old fixture omitted the native error envelope.

The APK transport now recognizes only the three known read-readiness errors
inside that structured envelope. It still rejects JSON-RPC errors, unrelated
errors, failed navigation and changed handset sessions. The native-envelope
regression failed before the repair; all 23 reader tests passed afterward.
No Android, Win or server executable was rebuilt for this transport-only fix.

The corrected scripts were copied into a durable, content-addressed local MCP
runtime. Existing Codex tool processes do not automatically adopt configuration
changes: this task's already-loaded scope still advertised Win only. New MCP
processes must use the new configured runtime and APK endpoint.

## Remaining Device Failure

The repaired adapter opened the ChatGPT surface, but the read timed out after
45.3 seconds with zero pages. The live native state reported adapter 437,
`adapter_current=false`, `adapter_generation=0`, `bridge_state=connecting`.
The user confirmed a blank/loading page. A single refresh did not establish a
ready reader. No private conversation content was exported or saved.

From inside the APK, both the unauthenticated ChatGPT homepage probe and the
Yilong health probe returned HTTP 200; ChatGPT TCP port 443 was reachable.
These probes do not prove the WebView's authenticated session, asset loading or
private API access. The user subsequently identified that the VPN had been off;
that explains the reported connectivity interruption without implying the
current authenticated reader has recovered. A fresh device read remains needed.

## Audit of the Referenced Private Integration

The referenced Codex task, **语音问题 (2)**, contains prior private transport,
native media, structured content and group-AI work. Its final repair commit
`d16a74a` is an ancestor of this baseline. Existing capability records also
document independent private HTTP sending in specific accepted scopes; these
are not evidence of a wholly WebView-free identity/bootstrap implementation.

Current architecture authority is the
[private integration playbook](../web-ai-private-integration-playbook.md).
It separates page-local private HTTP, reviewed runtime commands, native media
and independent native HTTP. WebView identity is intentional; DOM/composer
readiness must not become a universal admission gate.

The personal MCP reader already uses the existing private auth context and
bounded JSON request modules to GET the authorized conversation and all older
pages. It does not scrape visible bubbles. However:

1. `ChatGptWebOperationReadiness` requires `adapterCurrent` before admitting
   `chatgpt_read_conversation`; `ChatGptWebConversationRead.read` independently
   repeats that requirement. Thus this private read is still coupled to full
   page-adapter readiness. Removing composer dependence alone is insufficient.
2. The APK reader injects its two projection/reader assets and relies on the
   larger page bootstrap for auth/JSON modules. Win already has a separate
   read-only bootstrap. Session ownership and origin checks must be preserved
   when making APK read admission independent of UI bootstrap.
3. The MCP projection returns supported text and attachment metadata with
   explicit gaps. Existing native rich-history, writing-block and file/image
   capabilities are not automatically exposed by that projection. Their owners
   should be reused rather than replaced by screenshots or another downloader.

The intended integration is a scoped MCP read through the existing provider
session, private history and content modules, retaining account/document/cursor
ownership. Full content coverage, APK authenticated reading and a real Claude
invocation remain unverified. Wireless connection and the scoped transport fix
are complete; the original cross-client reading requirement is not complete.
