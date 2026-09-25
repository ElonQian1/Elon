---
version_status: current
reviewed_at: 2026-09-25
---

# Personal conversation rich reader acceptance

This follows the [wireless/private-integration audit](personal-web-conversation-apk-wireless-audit-20260925.md).
Only structural evidence is recorded. No conversation bodies, account identity,
credentials, signed URLs, phone serials or LAN addresses are retained.

## Existing mechanism and completed wiring

The referenced task **语音问题 (2)** already implemented rich history, writing
blocks, image/file ownership and private downloads; its repair `d16a74a` is an
ancestor of this work. Those capabilities were present. The missing portion was
their exposure through the personal conversation MCP reader.

The reader now reuses private history/text-block projections, file authorization,
image-pointer policies and the existing bounded byte-transfer protocol. WebView
retains local login identity; reads use page-local private HTTP, not visible chat
bubble scraping. This is not a claim of fully WebView-independent identity.

- Win/APK load the same production history/media dependencies.
- Code and saved writing blocks retain structured representations alongside text.
- Attachment handles bind one authorized conversation, device, account and
  snapshot. Bytes are checked and chunked; no provider locator or credential is
  returned. Per-file limit: 8 MiB; page-local asset cache: 32 MiB.
- MCP emits supported binary images as image content, small UTF-8 files as text,
  and PDF/other files as embedded resources. Resource interpretation remains a
  client capability, not a consequence of receiving bytes.
- Both Codex registration and the Claude/Codex CLI runtime package include the
  attachment module. Registration preserves this reader's existing APK endpoint.
- APK admission no longer requires a ready composer/layout adapter. Every
  authorized poll renews the existing bounded WebView execution lease.
- The desktop host validates both text and asset pages against the exact
  request, conversation, revision, index, offset, size and continuation.

## Release and verification identity

| Component | Accepted publication |
|---|---|
| Win installed runtime | `0.3.69+b3945a7486e0e269b46ede1b51d5f1a477e952aa` |
| PC frontend | `91f704c4b58821330fd2b1bd2ef08f06051819e7` |
| Existing compatible backend | `0.3.1775`, `e264ea10372d79f9f299ab5f2c313300c57fac74` |
| Current APK | `1.1.1813 / 1813`, source `91f704c4b58821330fd2b1bd2ef08f06051819e7` |
| APK SHA-256 | `673052e043ba57fe3a127395e91292bab3e39225287d153fae43aa810e4ac98b` |

Win was activated through the fixed exact-release `update_and_restart` MCP
action. The installed node's release identity was read back successfully; its
durable activation state is `activated`, and the remote outbox is `synced`.
The PC frontend was published independently and refreshed through `reload_page`.

Offline checks passed: 154 initial JavaScript cases plus the added canonical APK
error regression; the final focused script checks; 25 desktop host tests;
targeted Android admission/background execution tests; Win Cargo check and CLI
packing harness; frontend typecheck/build/lint; source and document guards.

## Actual account acceptance

| Case | Observed result | Verification |
|---|---|---|
| Win, fresh registered stdio MCP | 23 pages, 45 blocks, 25443 text characters; all 8 attachment references returned typed PNG images | device_verified |
| Win independent images | 4 unique byte hashes; duplicate source references are not additional images | device_verified |
| APK 1812, registered Xiaomi over wireless ADB | Same 23 pages, 45 blocks and 25,443 characters; first two images read; later downloads stalled after the idle execution pause | partial |
| APK 1813 execution lease repair | Compiled, tested and published; phone disconnected before postflight installation | deferred |
| Registered HONOR | Offline during postflight | deferred |
| Saved writing/code, ordinary/shared text and PDF files | Production modules exercised with synthetic data, including identity/redirect/size rejection | offline_verified |
| Actual Claude model invocation | Claude CLI absent from this workstation; common MCP/configuration path verified | deferred |

Win and APK 1812 plaintext hashes matched:
`6559110de18bbe27528926c55618421a557902caead2a682ff080ed7469f70fb`.

Win attachment evidence (deduplicated):

| Bytes | SHA-256 |
|---:|---|
| 109562 | `ec7ffab2925024e9f0f01b8b5812b77d31edca074604db5f51bc66abcf5ec16f` |
| 38744 | `b6edf85ce97dc229d80bcc7340a8642b4bbb958ee06a698aba98dc944aec7387` |
| 202592 | `3269ccd86c866adf5753e6ff783544f8a4371aa3e979480646f451ed8e43efd7` |
| 101756 | `1e1cc5172618e6cda651b5c36ef907cbe33ff08f6f7ce583a634966647f69a18` |

The snapshot reports 90 candidate messages and 32 messages with output blocks.
It retains `text_complete=false` and `unsupported_message_content`; this does
not establish complete parsing of every source record. Snapshot
`attachment_bytes_not_read` describes the initial text snapshot; separate asset
receipts prove that all Win image references were subsequently read.

## Runtime defects found and repaired

The first Win media attempt reached native bytes, but the workbench result parser
accepted only text snapshot schemas. The scoped asset validator fixed this; the
next actual account run read every image.

APK 1812 bypassed the existing execution-lease callback. The background controller
pauses idle WebView execution after 10 seconds, so a download could remain pending
and later assets reported busy. The reader now calls the same execution callback
on each poll; stopping reads releases the lease through the existing idle policy.

The user's earlier blank page was associated with VPN being off. After recovery,
native state showed an authenticated official conversation with a ready adapter.
That network incident is distinct from the execution-lease defect.

## Pending device acceptance

APK 1813 postflight found both registered phones offline. An explicit reconnect
to the previously verified Xiaomi endpoint timed out and the ADB device list was
empty. The user has been asked to restore the same Wi-Fi or authorized USB link.
The next step is install-with-data-preserved, version readback and one complete
fresh MCP text-plus-image run on 1813. No successful 1813 phone run is claimed.

The durable Codex MCP registration exposes connect, scope, read and asset tools
to new processes. Already-running Codex tools remain on their old snapshot until
the client reloads tools or starts a new task.
