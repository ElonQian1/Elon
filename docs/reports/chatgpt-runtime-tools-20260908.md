# ChatGPT tool runtime delivery and acceptance

## Current result

The tool capability is implemented and released, but **not device accepted**.
Do not mark `android_chatgpt_private_composer_tools_state_v1` completed or repeat
the accepted guest text/voice tests as a substitute for this missing evidence.

| Checkpoint | Result |
|---|---|
| APK 1559 hot new-chat | `runtime_new_chat:ready`, zero messages/ready in 1,898 ms; document generation unchanged; no modal approval |
| APK 1559 guest tool inventory | Search present; Create Image and research are login upsells; no selection |
| APK 1561 private tool catalog | Failed before selection; final `list_composer_tools` receipt failed on the existing DOM menu path after about 4 seconds |
| APK 1561 public asset inventory | Shared, conversation and composer match the inspected September 7 source profile |
| Follow-up offline contracts | 234 Node cases passed; 7 Release Kotlin tests passed with zero failures/errors/skips |
| APK 1563 delivery | Release build, remote hash/size, publication and replacement installation passed |
| APK 1563 acceptance | First attempt rejected before tool dispatch because MainActivity was not bound; after normal launch the readiness guard observed keyguard and stopped |

Neither 1561 nor 1563 acceptance selected a tool, sent text, recorded audio,
changed accounts, cleared application data or exported credentials. The 1561
production home/awake lease was restored. The first 1563 attempt restored its
awake lease; its home action was best-effort while MainActivity was unbound, not
a verified rendered restoration. The later locked attempt changed no settings.
The user was asked to unlock/open the normal app before continuing.

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

## Evidence and next operation

Log stems in the Git common directory's `ai-command-logs`:

- `runtime-tools-1561-terminal-20260908-20260908-113427-566`
- `runtime-tools-context-tests-20260908-20260908-114241-699`
- `runtime-tools-context-android-20260908-20260908-114645-181` (306.2 seconds)
- `runtime-tools-context-release-20260908-20260908-115452-659` (363.8 seconds)
- `runtime-tools-1563-device-20260908-20260908-120147-994`
- `runtime-tools-1563-ready-device-20260908-20260908-120448-642`

After the user is ready, use the existing native MCP on the trusted Xiaomi.
Request `chatgpt_list_composer_options` with `section=tools` once; poll the exact
`command_receipt.request_id` in `chatgpt_web_mcp.command_requests`. Only after a
successful `official_tool_runtime_v1:accepted` catalog may a reversible Search
on/off check use the returned handles. Confirm the original selection is restored
and the document generation is unchanged; do not automatically replay a failed
write. If listing fails, request `chatgpt_private_protocol_probe` with mode
`composer_tool_context` and read that request's terminal result. This command
reports the previous capture stage without importing or starting another capture.

`social_chat.web_chat_last_command` is not the diagnostic ledger: private probes
are intentionally excluded. Earlier short inspections cancelled pending menu
requests while restoring home; the request-ID terminal checkpoint, not those
cancellations, is the failure evidence. No screenshot or arbitrary page script is
needed for the next operation. Google and the overall Goal remain unfinished.
