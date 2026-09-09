# ChatGPT operation readiness

## Scope

The production native consumer port and MCP share `ChatGptWebMcpActions`.
`ChatGptWebOperationReadiness` classifies admission at that shared boundary;
composer readiness is not a global prerequisite for all commands.

| Requirement | Operations | Admission |
| --- | --- | --- |
| Local | State, native view selection, recovery, native download cancellation | No webpage prerequisite |
| Cached directory | Conversation cache, navigation cache | Immediate; mark stale document data and omit obsolete clickable option/feature handles |
| Current document | Context/controls, existing conversation navigation, model/tool discovery, stop generation, library request cancellation | Current document binding, supported origin, no explicit authentication page; no composer prerequisite |
| Directory read | Conversation/project directory refresh | Current document; existing private reader acquires identity independently of UI observations |
| Account read | Library/file lists and private download resolution | Current document; private transport validates account context and exact handles; no composer or UI-authentication-flag prerequisite |
| Account mutation | Conversation pin/archive/rename/move/delete, share creation/list/revoke, library rename/trash | Current document; existing private owner validates account, target, confirmation and operation-specific context. Share listing is read-only and needs no write confirmation. No global composer prerequisite |
| Composer transaction | Existing send, draft, attachment commit and voice commands | Preserve existing bridge/current-generation admission and command-specific validation |

Unknown actions are rejected. Every published action must belong to exactly one
reviewed category; a regression test enforces coverage. Opening cached UI does
not submit a network request or create a command receipt. Dispatch acceptance
is not provider success: the existing receipt still supplies the final result.

## State ownership

The adapter generation protects against executing an old command on a newer
document. It is not proof that the composer is rendered or that an HTTP request
will succeed. Authenticated identity is separate from the composer and from
provider rate limits. Private readers own credential acquisition, bounded waits,
account binding and failures. A false UI `authenticated` flag is not evidence
that a cached/private request context is unavailable. The shared entry must not
block that acquisition before the private reader has attempted it.
Document-bound commands need not wait for an independent chat snapshot event;
the adapter enforces the live WebView origin for dispatch. Account reads still
require valid credentials inside the private reader, not a UI identity snapshot.
Explicit authentication/unsupported-origin
evidence is never treated as a missing composer.

Persistent directory caches keep their existing account-clear lifecycle.
Page-local control IDs, model options, feature handles and context remain
document-bound. Only cached directory metadata is readable across a temporary
adapter gap; old context is not misrepresented as the newly selected chat.

Direct background model discovery and project-membership reads also use the
document/directory policies instead of the session-wide `READY` flag.

## Preserved boundaries

This is a scheduling/admission correction, not a replacement private protocol.
Same-origin private requests still use the resident WebView identity channel.
Individual adapters may still need official runtime bindings or specific DOM
controls for operations they have not migrated. Removing an outer wait does not
invent those bindings or authorize an unknown request.

Send/draft transactions, new-chat confirmation, attachment preparation/commit
and voice admission are intentionally unchanged. They need separate operation-specific evidence before their remaining
composer/runtime coupling can be removed. Do not convert all commands to READY,
replay writes, clear cookies, or change the working dictation/voice implementation.

Pin/archive/rename/move use their existing private PATCH owner, not the official
composer. Their account, credential-owner and document checks remain live across
credential acquisition, server acknowledgement and read-only reconciliation.
Changed context never updates a replacement cache or replays a write; failure
cooldowns do not carry into a different account/document.

Share/delete and library mutations now reach their existing private owner while
the composer is unavailable. Previously the shared native/MCP boundary rejected
them with `bridge_not_ready` before the operation could validate its real needs.
This correction does not remove their inner protections:

- Share list/revoke bind the live account/document; revoke also requires an
  unexpired selection ticket and confirmation. No composer is required.
- Creating a share still binds the current non-streaming conversation and its
  observed official runtime state. Its private contract retains the current
  composer check; this batch does not claim independent share creation.
- Deleting a different, observed conversation does not require a composer.
  Deleting the current conversation still refuses unknown/busy composer state,
  drafts, attachments and active capture, both before and immediately before
  the write. Ambiguous writes are reconciled, never replayed.
- Library rename/trash still require the exact observed handle, supported
  capability and explicit confirmation. Their private selection remains
  account/document-bound. Attaching library files is still a composer transaction.

No credential, endpoint, timeout, fallback, voice or dictation implementation was
changed by this classification correction.

Global directory refresh now reads bounded ordinary-history pages and owned
project metadata through the private identity owner; project reads follow their
cursor without waiting for the composer. Only a confirmed terminal project page
can replace that project's cached rows. Global cache completeness remains false
because separately fetched project histories must be preserved. Missing
identity and network failures remain recoverable request failures, not missing
DOM capability. Explicitly disabling the private transport preserves the legacy
official-directory route; a transient private failure does not silently use it.
Protocol evidence, limits and verification boundaries are in
[the pagination report](reports/chatgpt-directory-pagination-20260909.md).

## Verification

Implementation: shared admission policy, native wiring, scoped failure reasons.
Regression cases cover missing composer, stale document, unknown identity,
explicit login, foreign origin, guest access, cached directory, expired download
handles, production consumer dispatch, stop/cancel and unchanged write checks.
Build, release and device acceptance evidence is recorded separately in
[the batch report](reports/chatgpt-operation-readiness-20260909.md).
The follow-up phone evidence and remaining refresh boundary are recorded in
[the acceptance follow-up](reports/chatgpt-private-admission-acceptance-20260909.md).

### Account-owner admission follow-up, 2026-09-09

`code_status=implemented`, `verification_status=offline_verified` for the
share/delete/library-mutation admission correction. Release Kotlin compilation
and 49 focused Android unit tests passed, including real production consumer
dispatch with `CONNECTING` and `composerReady=false`. The four existing private
delete/share/shared-links/library-mutation suites passed all 129 cases. The
library suite's obsolete dispatcher-version literal was replaced by an actual
older-version upgrade and same-version reuse check.

The Xiaomi wireless transport was offline and a bounded reconnect timed out.
No new live mutation was attempted, no conversation or file was deleted, and
project-page error diagnosis remains deferred. Offline dispatch acceptance is
not a server acknowledgement or a rendered-UI acceptance result.

Normal Release **1.1.1613 (1613)** is published from
`98267d022b6981c5da3576adb325b68f2f6ef63e`. The release manifest and local signed
APK agree on 40,059,473 bytes and SHA-256
`53ea69ae997b258f08024adaf0fa09a308e9f14ca48ab58406cbf3f5723d6293`.
The post-release device list remained empty, so installation and the new
composer-unavailable UI scope are deferred. Earlier accepted mutation scopes
remain valid historical evidence, not acceptance of this correction.

When the existing phone returns, use only isolated fixtures to check share-list
access and confirmed library mutations through production UI without a ready
composer, then restore the original view/draft. Do not repeat completed private
protocol research, clear login, or delete personal conversations to manufacture
an acceptance case. Current-chat share/delete runtime protection remains a
separate boundary; project attachment/send acceptance is still open.
