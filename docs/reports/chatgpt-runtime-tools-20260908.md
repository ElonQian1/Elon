# ChatGPT tool runtime delivery and acceptance

## Current result

The Search toggle is implemented, production-default and **device accepted** on
APK 1565. Record `guest_search_toggle_v1` under
`android_chatgpt_private_composer_tools_state_v1` as completed and reuse it without
repeat research unless a regression appears. Image/account-scope acceptance is
still pending; do not mark the combined tool capability fully completed.

| Checkpoint | Result |
|---|---|
| APK 1559 hot new-chat | `runtime_new_chat:ready`, zero messages/ready in 1,898 ms; document generation unchanged; no modal approval |
| APK 1559 guest tool inventory | Search present; Create Image and research are login upsells; no selection |
| APK 1561 private tool catalog | Failed before selection; final `list_composer_tools` receipt failed on the existing DOM menu path after about 4 seconds |
| APK 1561 public asset inventory | Shared, conversation and composer match the inspected September 7 source profile |
| Follow-up offline contracts | 234 Node cases passed; 7 Release Kotlin tests passed with zero failures/errors/skips |
| APK 1563 delivery | Release build, remote hash/size, publication and replacement installation passed |
| APK 1563 acceptance | First attempt rejected before tool dispatch because MainActivity was not bound; after normal launch the readiness guard observed keyguard and stopped |
| APK 1563 after unlock | `composer_tool_context:composer_detached`; tool listing failed before any selection |
| Anchor follow-up | Context v3 accepts the official test id if the DOM id is overridden; 238 focused Node cases passed |
| APK 1565 delivery | Release build, remote hash/size, publication and replacement installation passed |
| APK 1565 Search acceptance | Native `social_ai`: on 165 ms, off 180 ms; both `official_tool_runtime_v1:accepted`; no document reload, zero messages |

Neither 1561 nor 1563 acceptance selected a tool, sent text, recorded audio,
changed accounts, cleared application data or exported credentials. The 1561
production home/awake lease was restored. The first 1563 attempt restored its
awake lease; its home action was best-effort while MainActivity was unbound, not
a verified rendered restoration. The later locked attempt changed no settings.
The user was asked to unlock/open the normal app before continuing.
After unlock, 1565 executed exactly one on/off round trip and restored the original
disabled state. The helper restored its awake lease and home; a separate read
confirmed `active_surface=conversation_home` and `input.has_text=false`. No audio,
message send, login, consent approval or account change was performed.

## Changes and artifact

`805ed8d07` added the current official `Yg`/`n_` tool getter/setter aliases, shared
committed conversation ownership, independent Search/image eligibility and the
bounded compiled-menu context. It did not create a parallel account/tool store.

`a4b742d76` fixes the assumption that this owner has only one compiler cache:
other-sized caches are allowed, while the exact 265-slot candidate must remain
unique and bound to the same hints/owner. It also adds an allowlisted, read-only
capture-stage diagnostic. This is a verified code gap, **not a proven root cause
of the phone failure** until the new stage receipt is observed.

APK `1.1.1563 (1563)`, adapter `301`, source `fe9d7676a`:
SHA-256 `643eda3f0af13d70999d76d0de481a5e1dab846a491b63d41afef7b181b9bdb0`.
It preserves the intervening Binance changes. No independent proxy, audio,
subtitle, dictation or text-submit implementation was changed in this follow-up.

The next source `1b2676914` corrects the id-only trigger anchor and binds the
expanded-menu guard to the same node. APK `1.1.1565 (1565)`, adapter `302`, has
SHA-256 `389fc22c99296806482a91a2a59345222d9db48b99993f105117f286d98b5978`.
Release build succeeded in 6m34s; the full publication/install took 449.1s.
The successful 1565 case isolates that anchor as the observed 1563 failure; it
does not retrospectively prove the compiler-cache change was the phone root cause.

## Evidence and next operation

Log stems in the Git common directory's `ai-command-logs`:

- `runtime-tools-1561-terminal-20260908-20260908-113427-566`
- `runtime-tools-context-tests-20260908-20260908-114241-699`
- `runtime-tools-context-android-20260908-20260908-114645-181` (306.2 seconds)
- `runtime-tools-context-release-20260908-20260908-115452-659` (363.8 seconds)
- `runtime-tools-1563-device-20260908-20260908-120147-994`
- `runtime-tools-1563-ready-device-20260908-20260908-120448-642`
- `runtime-tools-1563-unlocked-device-20260908-20260908-121651-221`
- `runtime-tools-anchor-tests-20260908-20260908-122440-959`
- `runtime-tools-anchor-release-20260908-20260908-122810-235`
- `runtime-tools-1565-device-20260908-20260908-123633-669`

The accepted Search case needs no repeat. Next tool acceptance requires an
account where the official menu actually offers image creation; the observed guest
image entry was an upsell, not authority to enable it. Keep private listing and
selection receipts tied to the exact `command_receipt.request_id` in
`chatgpt_web_mcp.command_requests`. On a new listing regression, the read-only
`composer_tool_context` probe reports the previous capture stage without another
import/capture. No automatic failed-write replay is allowed.

`social_chat.web_chat_last_command` is not the diagnostic ledger: private probes
are intentionally excluded. Earlier short inspections cancelled pending menu
requests while restoring home; the request-ID terminal checkpoint, not those
cancellations, is the failure evidence. No screenshot or arbitrary page script is
needed for the next operation. Google and the overall Goal remain unfinished.
