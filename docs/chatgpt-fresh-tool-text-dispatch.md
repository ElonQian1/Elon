---
capability_id: android_chatgpt_fresh_tool_text_dispatch_v1
implementation_status: implemented
verification_status: offline_verified
production_default: false
scope: authenticated_existing_personal_text_search_or_picture_v2
---

# Fresh Tool Text Dispatch

September 13 source batch extending [the accepted plain-text sender](chatgpt-fresh-text-dispatch.md).
This is not completion or promotion of tool-bearing sends. Existing production
Search/Create Image controls still use their accepted sender unless the scoped
candidate is explicitly admitted. No UI, transcript store or second send queue
is added, and accepted plain-text defaults remain unchanged.

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

On the grouped APK, use one owned existing personal conversation, select Search
with the real native control, arm one trial and click the native Send button.
Require a unique user turn, actual Search result/references, fresh-owner delivery
and terminal-history reconciliation. Repeat once for Create Image, requiring the
native generated-image result and matching history, not just HTTP acceptance.
Restore the original conversation, draft and tool selection afterward. Promote
each passed scope separately; do not widen to new chats, attachments, projects,
temporary chats, Study/Canvas or unobserved model-specific overrides. A failure
after dispatch is reconciled read-only, never resent as an acceptance retry.
