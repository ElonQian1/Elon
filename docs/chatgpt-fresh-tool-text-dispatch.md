---
capability_id: android_chatgpt_fresh_tool_text_dispatch_v1
implementation_status: implemented
verification_status: offline_verified
production_default: false
scope: authenticated_existing_personal_text_search_or_picture_v2
---

# Fresh Tool Text Dispatch

September 13 source batch extending [the accepted plain-text sender](chatgpt-fresh-text-dispatch.md).
Current scope: [existing-personal Search](chatgpt-fresh-search-text-dispatch.md)
has a real native/independent-HTTP pass and narrowly scoped default admission.
Image and other combinations remain candidates, so this combined capability is
not wholly complete. No UI, transcript store or second send queue is added;
accepted plain-text defaults remain unchanged.

[September 14 device preflight](reports/chatgpt-fresh-tool-preflight-20260914.md)
passed on APK 1728. Its collapsed-composer stop and pending fixture were later
resolved without a send replay. The [subsequent device report](reports/chatgpt-fresh-tool-device-20260914.md)
records the actual Search pass, confirmed cleanup and remaining Image preflight
failures; the earlier report remains historical evidence.

September 14 acceptance-tooling batch: `chatgpt-fresh-tool-smoke-evidence.ps1`
combines owned send continuity with completed Web output and matching native
source-message parts. It rejects old replies placed after a new user turn,
truncated context, unknown streaming state, and changed conversation/provider.
`fresh-tools-evidence-old-reply-baseline-20260914-180822-683` reproduced the old
reply false positive; `fresh-tools-evidence-verified-20260914-181109-416` passed
18 synthetic checks. No tool request was sent on a device and no defaults changed.

## Evidence And Request Shape

Only the retained `web_20260912` public assets are parsed; downloaded website
code is not executed. Hashes and exact AST assertions are in
`scripts/test-chatgpt-text-dispatch-public-evidence.cjs`.

- The shared `uG` enum identifies `search` and `picture_v2`. The reviewed
  composer `$g -> sR` reads the selected hint; `aR` projects a single active
  non-connector hint when no custom agent is selected.
- `FB -> AB` prepares with the selected `systemHints`. `CJ/SJ` keep the selected
  hint in user-message `metadata.system_hints`.
- Search is removed from completion `systemHints` before dispatch and replaced
  by `forceUseSearch=true`. `AB` writes `force_use_search=true` and
  `client_reported_search_source=conversation_composer_web_icon`. The selected
  `search` value remains in the user message, not the final top-level hint list.
- Picture keeps `picture_v2` in both final hint lists. These selected-tool sends
  also use the observed `enable_message_followups=true` field.
- `VZt` obtains security material from the transformed completion metadata.
  Preparation, security and final body must not share one blindly copied hint
  list. Each command still consumes a fresh preparation/conduit only once.

## Ownership And Admission

Context v3 reuses the existing account/model-filtered private tool context once,
and requires matching document, identity, conversation, controller, shared props,
server route and model. Unknown/filtered tools, connectors, custom agents,
campaigns, attachments and restricted/busy contexts remain outside this scope.
Tool selection is part of the immutable command fingerprint. Subsequent checks
read the owned store without polling the DOM tool menu. Initial ownership still
depends on the committed composer/tool host; this is not composer-free admission.

Request v2 separately constructs preparation, security metadata and the final
request. It preserves the prompt, parent, model and privacy flags, and returns
fresh arrays so a caller cannot mutate the next phase's hints. Transaction v10
passes the per-command admission and security metadata through the existing
HTTP/stream/reconciliation chain. Tool or model changes before POST cancel that
attempt; uncertain writes cannot replay through the established sender.

The existing `fresh_text_trial_start` permits one candidate command for at most
120 seconds on the current document/account/route. It is consumed once, not a
persistent default. The page-local `__elonChatGptFreshTextToolsEnabled === true`
can explicitly admit this source scope; it is unset in production. The overall
private-transaction switch still applies. Failed pre-admission can claim the
existing sender exactly once before any independent preparation or dispatch.

## Verification And Delivery

- Public AST contract: 11 tests passed, zero skips,
  `fresh-tools-public-contract-20260913-104506-429`.
- Context, request, transaction, history, stream, stop, recovery and wiring:
  140 tests passed, zero skips, `fresh-tools-offline-20260913-104902-838`.
  Includes tool selection/identity drift, wrong model, filtered menu, cold
  admission, immutable request phases, single trial use, one POST and retained
  plain-text behavior. The first harness run attempted a new trial before the
  rejected owner's asynchronous cleanup; waiting one event-loop turn corrected
  the harness. It was not counted as a passing run.
- No new APK was compiled, published or installed for this source batch. Group
  it with the next Android release/production acceptance round. Previously
  installed 1699 does not contain these source changes.

## Remaining Acceptance

Reuse the completed existing-personal Search scope; do not repeat its sample.
For Create Image, resolve the observed tool-host ownership precondition first,
then perform one native send in the owned fixture, requiring the
native generated-image result and matching history, not just HTTP acceptance.
Restore the original conversation, draft and tool selection afterward. Promote
each passed scope separately; do not widen to new chats, attachments, projects,
temporary chats, Study/Canvas or unobserved model-specific overrides. A failure
after dispatch is reconciled read-only, never resent as an acceptance retry.

## New-Conversation Composition Audit

September 14, source `aec6b09c2`: new personal and owned-project Search/Picture
requests are already implemented by the existing context, request, transaction,
stream and history modules. They do not need another sender. Both new-conversation
and selected-tool admission must be explicitly enabled; the verified existing
personal Search default does not admit these combinations.

The pinned public-source AST checks confirm that the same reviewed `AB` body
projection carries the new root/model fields and selected-tool metadata. Search
is transformed only for final dispatch/security, while Picture keeps its hint.
`fresh-new-tool-contract-20260914-224225-893` passed 115 tests, zero skipped.
Fifteen added cases in the existing first-send suite cover the unpromoted default
boundaries, personal/project first-send and follow-up, acquired-ID Stop, failed
project navigation and loss of the first response before a server ID arrives.
They verify one preparation/POST per command, exact root/parent/mode/hint fields,
one server-ID adoption/navigation, retained native stream text, and no uncertain
write replay. This is evidence for composition of existing code, not a new API.

The integration fixture supplies synthetic identity, permission, HTTP and history
responses. It uses the actual context/request/transaction/stream/reconciliation
modules, but a synthetic text answer even for the Picture hint. It does not prove
live account eligibility, generated-image output, source citations, initial
composer-free behavior or production UI acceptance. Those remain device cases.
No Android source, default or APK changed in this test/documentation batch; 1737
already contains the audited implementation. Device ADB was unavailable, so no
request, installation, credential change or fixture-ledger mutation was made.
