# Native Library File Mutations

## Status And Scope

Capability `android_chatgpt_private_library_file_mutations_v1` implements ordinary
owned-library file rename and move-to-recently-deleted in the production native
file browser. Ordinary text-file rename and soft-delete are **completed, enabled
and device verified** in normal Release 1.1.1589 through the native UI, with
directory readback and original fixture/chat restoration. See
[the current acceptance report](reports/chatgpt-library-mutations-1589.md).
It does not implement permanent deletion, batch deletion, restoration, folder
creation/rename/move, project-file mutations or external-provider mutations.

The existing [library browser](chatgpt-private-library-browser.md), persistent
WebView identity and private JSON response owner are reused. No official-page
navigation, transcript reset, microphone change or secondary system implementation
is part of these commands.

## Observed Contract

Retained public asset `conversation-small-owrec55n6vm0ekcc.js` (2026-09-07), SHA-256
`7973d518b083f0f3e23905a279ed019378481bdbdd10fc0196afe9fc7b3b7d35`:

- `oCr.rename` performs `PATCH /backend-api/files/library/files/{library_file_id}`
  with JSON `{file_name: newName}`.
- `oCr.remove` selects `QSr` for soft deletion. `QSr` performs
  `POST /backend-api/files/library/files/{library_file_id}/delete_stream`, with
  `file_id`, optional `parent_directory_id`, `file_name` and `soft_delete=true`.
- `dqt` parses newline-delimited JSON, including a final JSON record without a
  newline. This is not the text-chat SSE protocol.
- `file.deletion.completed` is required; `file.deletion.error`, incomplete JSON,
  a progress-only response or empty HTTP success cannot confirm deletion.

Public source and synthetic responses establish an implementation contract, not
live-server acceptance. No credentials or private user examples are stored here.

## Ownership And Failure Semantics

- Native rows expose `canRename`/`canTrash` and opaque handles. Raw library IDs,
  backing-file IDs and authorization remain with the page owner.
- Mutation selection requires a current identity/document and an observed file
  from a fresh bounded catalogue entry. External, saved-entity and project rows
  do not advertise operations whose contracts have not been reproduced.
- The user confirms one rename or soft-delete operation. MCP uses the same typed
  `chatgpt_mutate_library_file` path with `confirmed=true`; forged/stale handles,
  unsupported operations and invalid names are rejected before dispatch.
- A single write may be active. The request ID is bound to exact parameters and
  context; duplicate delivery shares the same pending/final receipt, not a write.
- Writes use the existing 20-second, 64-KiB bounded HTTP reader. Rename requires
  HTTP acknowledgement; soft-delete also requires the explicit stream completion.
  HTTP rejection, timeouts and unknown results never trigger another endpoint or
  automatic write replay. Failed writes impose a 45-second cooldown.
- Active older reads are cancelled before writing. Confirmed changes update
  cached rows; unknown results retain them. Both expire catalogue freshness and
  require a read-only refresh. Identity/navigation changes suppress late updates.
- The native progress surface may be collapsed. Its receipt watcher stops after
  a terminal result or 30 seconds; closing the parent browser stops the watcher,
  not the already-dispatched server write. It never claims cancellation of a write.

Stable selectors: `web-chat-library-rename-input`,
`web-chat-library-mutation-confirm`, `web-chat-library-mutation-status`.
The browser retains `web-chat-library-browser` and existing file selectors.

## Verification

### Search Index Lag (2026-09-09)

Native rename on normal Release 1.1.1588 returned an acknowledged write. A fresh
request-bound `nodes?q=ELON` read still returned the old fixture name, while a
separate `nodes` directory read returned the renamed fixture. This is search
index lag, not proof that the PATCH failed. Immediate optimistic UI inspection
and even a refreshed search page are insufficient persistence evidence.

The current public asset `conversation-small-fka464yvjn19vebr.js`, SHA-256
`1c130659c30fda891471cc9c4a8a5c491b595a1ca219516f0f90b0304b9b3c6f`, retains the
same PATCH/body contract (`zSr.rename`) and NDJSON soft-delete (`jSr`). No
alternative endpoint was introduced.

Catalogue module 6 retains at most 128 recent acknowledged rename hints, with
16 previous names per file and a ten-minute lifetime. Only search results with
one of those previous names are reconciled. Unfiltered directory responses and
different remote names take precedence; identity changes clear the hints.
This adds no fetch, polling or DOM access, and does not claim to reindex search
results. Native UI acceptance must verify the exact field value and read the
file's directory independently; restore names before completing the test.

The four related Node suites passed 58 tests
(`library-rename-search-checks-20260909-052400-132`), including search lag, rename
round trips, remote changes, expiry and account isolation. The external semantic
UI runner also compiled and ran on the phone without replacing the APK.

Targeted JavaScript coverage includes official paths/bodies, cache reconciliation,
NDJSON completion/error/malformed frames, duplicate dispatch, concurrent writes,
confirmation/name validation, unsupported kinds, identity/navigation changes,
bounded response-reader integration and directory-dispatcher version upgrades.
Native tests cover strict boolean capabilities, confirmation and observed selection,
typed command wiring, unsupported destructive variants and receipt deadlines.

2026-09-08 results: 181 related JavaScript tests and 102 adapter/bundle syntax
checks passed (`library-mutations-related-20260908-223102-966`). Android Debug
compilation and 16 targeted JVM tests passed
(`library-mutations-native-20260908-222818-220`). Subsequent grouped Release 1.1.1579
was published and installed; see the [browser receipt](chatgpt-private-library-browser.md).
The user subsequently signed in. Release 1.1.1581 fixes the catalogue bridge and
passes request-bound native root/folder/cache/search reads. It does not establish
mutation success; the rendered-menu handoff remains unconfirmed. See the browser's
current evidence rather than treating account selection as a continuing blocker.

The ordinary-file acceptance above is now complete: one isolated fixture was
renamed and restored; only an explicitly confirmed disposable copy was soft-deleted.
No user file or permanent deletion was involved. Voice was not exercised during
this file-only round. Reuse the completed scope; do not repeat it without a new
regression. Folder and special-source operations remain outside this capability.
