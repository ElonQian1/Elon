# Bounded private protocol evidence

Status: published/installed `v1.1.1540`, 2026-09-06. This is diagnostic infrastructure, not a
completed uploader, downloader, private sender, or share/delete implementation.
The outstanding contracts remain in [the batch work list](web-ai-private-native-remaining-batch.md).

## Purpose

The installed release can expose native semantic commands but not arbitrary
WebView debugging. Its old research observer also truncates ordinary command
details to 160 characters. Neither limitation should cause repeated screenshots,
guessed request bodies, or an APK rebuild for each missing endpoint.

The existing request observer now supports a bounded, explicit capture command.
It uses the existing JSON body deadline helper and native command ledger, not a
second transport, HTTP client, continuous network logger, or test UI.

## Native command

Call APK MCP `ui_control` with:

```json
{"action":"chatgpt_private_protocol_probe","mode":"start"}
```

Modes are `start`, `read`, `stop`, and `clear`. The dispatch receipt contains a
request ID; inspect the matching `command_requests` result in `ui_state`.
Its expected web action is `private_protocol_probe`. The detail is validated
JSON with schema `elon.private_protocol_probe.v1`, not an unbounded raw body.
Check the receipt's success and schema before using it as evidence.

Start once, perform a small group of authorized official operations, then read
and stop. Do not poll repeatedly or automatically restart capture. Unrelated
chat actions and microphone state are untouched. No command performs an HTTP
request, selects a file, starts recording, sends a message, or changes a login.

## Limits and ownership

- Default inactive, with no capture timer or body read.
- Only same-origin `/backend-api/`, `/api/`, and `/ces/` fetch/XHR requests.
- Reuses the existing path sanitizer; removes query strings and normalizes
  encoded segments and long identifiers. Paths are structural diagnostics, not
  a credential source or proof that an endpoint is safe to replay.
- At most 12 records per start; automatic stop after 60 seconds, with an elapsed
  time check when resuming after background timer throttling.
- At most 64 KiB per JSON clone, a 2-second read deadline, 12 typed field paths
  per side, three nesting levels, and 12,000 output characters.
- Only method, normalized path, status, transport, body kind/read status and
  field names/types. No request headers, field values, filenames or file bytes.
- Binary and SSE bodies are skipped. JSON failure response types are observable
  without changing the status or consuming the original response.
- Stop aborts clone readers; late callbacks cannot update a stopped or restarted
  capture. Clear removes the probe's records, not older bounded native receipts.
- Native diagnostic receipts stop before the usual chat feedback/snapshot
  dispatch. Reading evidence does not refresh conversation content.
- No persistence, upload, retries, DOM scan, or new production menu. The Windows
  raw-response research feature is separate and is not enabled by this command.

## Verification

Focused Node tests cover inactive pass-through, request/response clone ownership,
error JSON, multipart metadata, bounded output, deadlines, stop/restart races,
origin filtering, XHR and diagnostic failures. Android tests cover strict result
validation, ordinary receipt bounds, MCP dispatch and document-generation gates.

2026-09-06 checks: 21 Node checks passed (20 focused cases plus the existing
research-probe suite); Android Kotlin compilation and 33 targeted unit tests
passed across evidence validation, MCP, existing protocol and file-index receipt
tests. The subsequent grouped Release build, publish and data-preserving install
passed on `v1.1.1540`; no microphone or actual protocol capture was performed.

These tests use synthetic fixtures and do not establish current vendor request
contracts. Device capture remains pending; see [delivery evidence](web-ai-private-native-remaining-batch.md#grouped-release). Endpoint
field types alone are not sufficient to implement credential issuance, upload
finalization, idempotency or transaction ownership; verify those semantics before
replacing any working production path.
