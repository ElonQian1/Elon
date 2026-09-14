---
capability_id: android_chatgpt_fresh_search_text_dispatch_v1
implementation_status: completed
verification_status: device_verified
production_default: true
scope: authenticated_existing_personal_text_search
---

# Independent Search Text Send

This narrows the implemented [tool sender](chatgpt-fresh-tool-text-dispatch.md)
to the scope actually verified on APK 1728: an existing personal conversation,
native Search selection and one native text send. Reuse this implementation and
sample; do not generate another Search acceptance turn without regression evidence.

## Default Admission

Context v14 and transaction v25 admit this scope without arming a research trial.
The existing `__elonChatGptFreshTextToolsEnabled=false` switch still disables it.
The global private-transaction and independent-send switches remain effective.

The route, current message tree, personal workspace, account, model-filtered tool
permission, immutable tool selection and writer ownership must all agree. The
personal scope is rechecked before dispatch, not just during initial capture.
This default does not include new conversations, projects (including canonical
`/c/` project routes), temporary chats, attachments, regeneration or other tools.
Those explicit research admissions and the accepted sender remain unchanged.

The production request uses the existing independent prepare/security/HTTP/SSE
and history-reconciliation modules. No second queue, DOM-click sender or Cookie
export is added. A request that may have reached the server is never replayed.

## Evidence And Release

[Device and cleanup evidence](reports/chatgpt-fresh-tool-device-20260914.md)
records one native send, one independent request, streamed references and matched
native output. The 15.4-second acceptance observation includes UI/MCP polling;
it is not a time-to-first-token or power benchmark.

`fresh-search-final-scope-guards-20260914-193349-451` passed 151 targeted tests,
including default opt-out and excluded scope combinations. Adapter 396 contains
the admission change. The device pass used the identical transport under a
one-shot admission on 1728; release/default-routing evidence is recorded in the
linked report, independently of that transport pass.
