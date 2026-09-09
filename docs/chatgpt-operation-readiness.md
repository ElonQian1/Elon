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
| Composer transaction | Existing send, draft, attachment commit, mutation and voice commands | Preserve existing bridge/current-generation admission and command-specific validation |

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

Send/draft transactions, new-chat confirmation, attachment preparation/commit,
conversation mutations and voice admission are intentionally unchanged in this
batch. They need separate operation-specific evidence before their remaining
composer/runtime coupling can be removed. Do not convert all commands to READY,
replay writes, clear cookies, or change the working dictation/voice implementation.

## Verification

Implementation: shared admission policy, native wiring, scoped failure reasons.
Regression cases cover missing composer, stale document, unknown identity,
explicit login, foreign origin, guest access, cached directory, expired download
handles, production consumer dispatch, stop/cancel and unchanged write checks.
Build, release and device acceptance evidence is recorded separately in
[the batch report](reports/chatgpt-operation-readiness-20260909.md).
