---
capability_id: android_chatgpt_fresh_new_conversation_text_dispatch_v1
implementation_status: completed
verification_status: production_ui_verified
production_default: true
scope: authenticated_new_personal_plain_text_with_memory_owner
pending_extensions: owned_project_text, temporary, tools, attachments, cold_page_bootstrap
---

# Fresh New-Conversation Dispatch

September 15 source extension: transaction 27 / input 5 / adapter 407 removes
the mounted-composer condition from the default personal plain-text scope.
The committed memory draft/controller is still mandatory. Actual owner-fixture
tests pass with the editor DOM absent, including first response loss and the
no-replay barrier. First send/follow-up passed on release 1743; that device run
did not remove the official composer. The [adapter 411 native acceptance](chatgpt-composer-unavailable-acceptance.md)
then passed with no usable official composer through the entire first send and
history readback; normal release 1748 is published and installed. This verifies the default memory-owner route, not cold-page
bootstrap with all runtime/editor modules absent. See [current compatibility batch](reports/chatgpt-runtime-bindings-20260915.md).

September 13 extension of the [accepted independent sender](chatgpt-fresh-text-dispatch.md).
It reuses its request ledger, security-aware page HTTP, stream decoder, native
projection, stop and history recovery. It does not add another sender or editor.
The personal native first-send and follow-up now pass grouped acceptance;
the project extension still requires an explicit trial. The accepted
existing-conversation default is unchanged.

Latest: [1720 complete personal first-send/follow-up and default promotion](reports/chatgpt-fresh-new-default-20260914.md).
Both native sends, exact conversation continuity, single-answer display and
restoration passed. Transaction 23 enables this personal scope with a bound
composer; release 1721 and its unarmed native default check passed. The broader
project scope is not promoted. The [earlier root/receipt repair](reports/chatgpt-fresh-canonical-root-20260914.md)
is reused, not repeated. The timeline below is historical evidence.

Earlier follow-up: [first-route reconciliation evidence](reports/chatgpt-fresh-first-route-20260913.md).
The 1708 pending fixture is now resolved by exact native/provider readback.
One first-send trial on 1709 then failed with `owner_changed`, not the prior
parent mismatch. Navigate-before-hydrate is offline-verified and installed in
1711, but exact pending recovery was interrupted before another native first-send
case. New-conversation production default remains disabled.

The next source batch (`62d81aff3`, context 10 / transaction 20 / adapter 381)
also starts that owned navigation on the first server-ID event, rather than
waiting for stream completion, and coalesces concurrent finalization. Its 267
targeted Node tests pass. [Grouped APK 1715](reports/chatgpt-private-grouped-1715.md)
now contains the exact current scripts; Release compilation and targeted Android
tests passed. Device acceptance remains deferred. The 1709 pending handoff is
still unresolved and unreplayed.

The [observed-route handoff](chatgpt-fresh-pending-observed-route.md) now retains
an exact provider route for future uncertain first sends and checks it before
reading directory pages. This tooling is offline-verified only; it does not
resolve the old handoff or change the disabled new-conversation default.

## Native Acceptance Failure On 1706

`fresh-new-native-ui-20260913-183220-505` used the actual native composer and
Send button on the installed release 1.1.1706, with the one-command trial armed.
There was no runtime seed: exactly one new-conversation candidate was sent.
The private request was dispatched and accepted; 31 owned stream events arrived,
including input-message, handoff and stream-complete events. History then stayed
at `parent_mismatch`, with `pending=true` and no confirmed reconciliation. The
native reply/follow-up/restoration case therefore **failed**, not completed.

The failure was first-send parent ownership in authoritative history and
subsequent runtime-tree reconciliation. At that point the checks accepted only
a direct `client-created-root` parent or its reviewed legacy empty-root
equivalent. The rejected shape was then unknown; existing-conversation success
did not prove this scope. The exact fixture was later resolved read-only, below.

The pending controlled fixture is retained locally at
`$GIT_COMMON_DIR/ai-acceptance-fixtures/fresh-text-pending.json`; it contains no
credentials and is not committed or printed. Use read-only resolution first,
not another send. The original route was not preserved by this failed run and
must not be guessed. At the end of the 1706 failure batch, no conversation
navigation, draft clearing, replay, WebView reload or APK replacement had
followed the failed write. The awake lease was restored. Production 1706 has no
app WebView debugging socket, so direct
CDP inspection was unavailable; its debugging/login policy was not changed.

The canonical smoke now has `-NewConversation`, rejects outstanding handoffs,
and requires private send receipts, owned stream events, history reconciliation,
native/server route agreement and unchanged prior user identities. An ambiguous
click/result blocks cleanup and saves a local handoff. Default new-conversation
scope stays disabled. The earlier `origin_not_idle` attempt sent nothing; it was
a harness error corrected to use main draft-presence plus the provider draft.

Focused script tests cover 15 negative send proofs, six identity cases and
nine cleanup cases; the existing retry/trial contract suite also passes.
That acceptance-tooling batch changed no Android source and built no new APK.

### Read-Only Parent Diagnostics

The follow-up adapter 377 adds `chatgpt_private_protocol_probe` mode
`history_parent`. It reads only the current ordinary conversation through the
reviewed runtime history loader with `shouldApplyResponse=false`. No composer
capture, send, replay, navigation or manual tree update is performed by the probe.
The five-second, single-flight read is bound to document, route, runtime profile
and account; owner changes discard the result. Existing network observers remain
unchanged when the command surface upgrades.

Only an allowlisted description of the latest user turn's ancestor chain crosses
the bridge: roles, ID categories (not IDs), structural equality/child-link checks,
hidden flags and bounded counts. Node and Android tests cover rejected scopes,
cycles, broken/missing links, late callbacks, timeout and private-field rejection.
This is diagnostic infrastructure, **not** evidence that an unexpected parent is
valid. New first-send admission and reconciliation stay unchanged pending the
exact failed turn's parent sample.

The diagnostic was published and nondestructively installed as `1.1.1707 / 1707`,
source `1c102939b0e6d9021151c46ed542ff0f70611099`, APK SHA-256
`9462278fb42c3997554516da4be53616df6a834eb174244bad640a1c999da6a8`.
`history-parent-release-20260913-192436-603` completed build, publication,
remote manifest/hash verification and Xiaomi installation in 472.3 seconds.
`history-parent-node-20260913-191727-956` passed 36 tests; the focused release
JVM run `history-parent-jvm-20260913-191729-314` passed 19 tests. Neither suite
had failures or skips. A prior Node run only required updating an existing
probe-version assertion to 26; that fixture mismatch was not a device failure.

The first current-history read, `current-history-parent-probe-20260913-193341-368`,
returned a bounded timeout. The warm read
`current-history-parent-probe-warm-20260913-193435-837` returned `observed` and
the allowlisted structural chain. It included hidden system ancestors and an
unclassified root with an empty parent. This was a **different conversation**,
not the pending first-send fixture; it does not authorize relaxing root or
branch ownership checks and is not a first-send acceptance pass.

Read-only navigation after the 1707 upgrade inspected three current-day cached
candidates; none matched both the pending user-message ID and exact controlled
prompt. An initially rejected third navigation succeeded after the previous
navigation settled; it was not proof of a deleted conversation. The later
provider directory status reported stale data and `directory_timeout`, despite
the native cache remaining usable. Therefore the bounded cache miss does not
prove that the accepted request was absent from the server. Its pending receipt
is retained, no message was resent, and the original route remains unknown.

The separate native first-page lookup did not execute: its external semantic
runner rejected `foreground_mismatch` while WeChat was foreground. The runner
diagnostic initially omitted `ComparisonFailure`, hiding that specific reason;
`fresh-native-directory-runner-detail-20260913-195552-074` identified it. This
is an acceptance-tool failure, not evidence that the first-page private read
failed. The awake setting was restored and no write or replay was attempted.
The pending fixture lacks the accepted server conversation ID, so rebuilding
the page-local writer cannot recover that ID. A future acceptance handoff must
preserve exact accepted ownership locally before replacing a pending page;
do not guess a conversation from title or activity date.

### Exact Fixture And Parent Repair

`fresh-pending-native-head-ready-20260913-200528-018` opened the actual production
native sidebar's complete-directory page using semantic controls. The first of
eight visible candidates matched both the preserved user ID and exact controlled
prompt. The completed answer was present. The read-only history diagnostic then
observed that turn's `user -> hidden system -> message-less root` ancestry,
with reciprocal single-child links and an empty parent on the server root.
No new message or replay occurred. The local handoff now records its resolved
path and completed readback; the user's pre-1706 route is still unknown.

Context/reconcile v8 admits this first-turn shape only under the existing exact
conversation, user, branch, project/privacy and active-owner checks. Before
applying history, it verifies a bounded chain of hidden text-only system nodes
ending at a message-less root, with matching node/message IDs, unique child
links, no cycles and no hidden preceding user turn. An arbitrary UUID parent
alone is still rejected. The verified chain captures structural IDs only.

After official hydration, the same IDs, parent links and hidden system roles
must exist in the canonical tree. The root must be its explicit root-role
message with an empty `parentId`, as verified in the pinned `Cx` tree contract.
Only that per-read verifier can authorize the canonical parent exception;
calling `reconciled` without the verified read cannot do so. Navigation must
still complete before the writer is released. Existing parent rules and all
uncertain-write replay restrictions remain unchanged.

The new fixture tests first reproduced the failure. Tests now cover ordinary
and owned-project first-send plus follow-up, changed canonical parents, visible
or non-system ancestors, broken/duplicate children, message-ID mismatch, cycles
and root replacement. Source coverage does not itself verify either device
scope. `fresh-parent-full-regression-20260913-201819-896` passed 303 Node tests,
with zero failures or skips, including pinned public-tree contracts and existing
writing-block save exclusions. `fresh-parent-recovery-contract-20260913-201555-411`
passed the existing send/continuity/cleanup checks plus 19 rejected readback
cases. Adapter 378 is prepared for the grouped release/native acceptance.

The acceptance runner can retain a previously resolved local handoff rather
than deleting it: before any fresh fixture, it requires current authenticated
provider and native routes, the exact original user ID/prompt, a single user
turn and the completed answer in both views. It archives the unchanged receipt
before a later uncertain fixture can replace it. A stored boolean alone cannot
permit a new test, and the previous message is never resent.

## Evidence

### Pagination Repair After 1708

`fresh-parent-release-20260913-202803-661` built, published and installed
`1.1.1708 / 1708`, source `13ece716a`, in 481.2 seconds. APK SHA-256:
`71b5f3577bdd95036308969a5274e2e5a736de5cfb30e51414eb6b16c7c72cce`.
Before the new trial, the previously resolved fixture passed exact live
provider/native readback with adapter 378. The original handoff was archived.

`fresh-parent-native-first-followup-20260913-203835-433` then failed after
105.9 seconds: one native first-send click, accepted private HTTP, 31 owned
stream events, but history still reported `parent_mismatch`. No follow-up
was sent, no replay occurred, and the awake setting was restored. A new
pending handoff preserves the exact submitted user/prompt and prior route.
Complete native reply/identity and restoration were not verified. A later
read-only recovery preflight rejected `unresolved_active_writer` before
opening the directory; do not treat message counts as exact reply proof.

Further inspection of the same pinned shared bundle found the missing
contract: export `mi/qHt` creates `paginated-root:${clientThreadId}` and
`RHt` constructs a linear mapping from `messagesLeafToRoot`. The prior
generic root identifier filter rejected its colon. The earlier diagnostic
classified this as `other`; it did not prove an arbitrary server root.
This source evidence explains a concrete uncovered shape, not a new device pass.

Reconcile v9 recognizes only the exact current conversation's pagination
root. Its private page metadata must have `cursor === null`, matching server
leaf and oldest message, and a unique complete message list with matching
mapping objects, parents and children. Missing metadata, an older-page cursor,
another root, duplicates, reordered messages and inconsistent edges fail
closed. The canonical-tree verification still runs afterward. The history
diagnostic now distinguishes owned pagination roots and completeness using
booleans and classes only, never IDs, content or cursors.

`fresh-pagination-parent-baseline-20260913-204658-771` reproduced the missing
pagination case. `fresh-pagination-final-regression-20260913-205108-289`
passed 306 tests with zero failures or skips, including the pinned pagination
constructor and normal/project first-send plus follow-up composition.
New-conversation production default remains disabled until native acceptance succeeds.

`fresh-pagination-release-20260913-205445-095` published and installed
`1.1.1709 / 1709`, source `08339aa2a`, adapter 379, in 449.8 seconds. APK SHA-256:
`97d9da026d2bca7398db61baf3ea5b7e53c01e28928c8a963ef9e9890bf108a0`.
The pending handoff was copied and byte-verified before upgrading; it is still
retained and blocks another fresh acceptance send. No cookies or app data were cleared.

Read-only lookup on 1709 did not resolve that handoff. In landscape, the native
directory reported more pages but exposed no list rows. A bounded row-layout
wait alone did not fix this. A temporary portrait lease exposed eight rows;
none matched the exact pending user ID and prompt. These bounded misses do
not prove the message is absent from the server. The native directory's
landscape layout is a separate remaining UI gap; the acceptance harness now
waits for list-child publication and distinguishes an explicitly empty page.

`fresh-pagination-portrait-acceptance-20260913-210825-330` restored rotation
and the awake lease, but its temporary wrapper then failed to import the
runtime helper in the parent scope. This is a harness error, not a private
request failure. It had sent no new candidate or follow-up. The lookup itself
had already failed to match the handoff, so fixing the import alone would not
authorize another send. The 1709 repair is offline-verified and installed,
not a new native first-send pass. Continue with bounded read-only fixture
resolution, then one first-send/follow-up case; do not replay the old prompt
or assume the first eight visible rows are the complete provider directory.

Only the retained, hash-pinned `web_20260912` public sources are parsed; downloaded
bundles are not executed. `test-chatgpt-text-dispatch-public-evidence.cjs` records
the shared/conversation/composer hashes and asserts these contracts:

- New conversations have a registered `WEB:` client owner and the exact
  `client-created-root` parent. The request omits `conversation_id`; it must not
  invent a server ID or substitute an arbitrary root UUID.
- Shared `IP/H1e` binds the server ID to the same conversation object, moves the
  registry entry and updates the client/server mapping atomically. `gY/QJ`
  identify and resolve the provisional owner. No manual store replacement.
- Composer `A1t` uses `Jsn/DDn` for the first requested default model and its
  remembered selection. Dispatch carries `requested_default_model` and
  `one_off_model_override` when present; preparation does not invent that field.
- `qHt` navigates through the official router, including canonical project
  routing. Its navigation argument can be a guarded callback (`r_n`); `HK`
  supplies the entry key needed to reject a changed navigation owner.
- The history callback receives raw data before `oQe` normalizes a legacy empty
  root to `client-created-root`. Only that exact root exception is supported.

Binding v23 exposes these helpers on this reviewed profile only. Request v4,
context v5, owned stream v2 and transaction v12 consume them. Existing runtime
send, projects, tools and their acceptance boundaries remain separate.

## Ownership And Recovery

Admission requires a normal, non-temporary personal workspace; an empty,
registered new conversation; its current model/effort/tool; no attachments or
competing writes; and an unchanged document/account/navigation entry. Project
roots reuse the [existing project scope and authorized headers](chatgpt-fresh-project-text-dispatch.md).
Unknown runtime profiles, shared/business contexts and configured continuations
are not silently treated as ordinary first sends.

The private request owns its decoded HTTP/topic stream. A matching input message,
an assistant message or a reviewed resume control may bind the returned server
ID once. Other metadata, another registered owner, another user message or a
later different conversation ID cannot authorize adoption. The existing SSE/v1
decoder publishes text to the native session after identity binding.

The stop request reads the acquired server ID, not the original null snapshot.
Before the ID arrives, it cannot consume the one-shot stop conduit. Authoritative
history must match the submitted user/parent/branch, project and privacy state.
For a non-temporary first send, owned server-route navigation now precedes
history hydration, following the pinned first-response handler. The same
identity/branch checks still guard history application and writer release.

Delayed project navigation callbacks expire on cancellation, timeout, changed
identity or route; a late result cannot pull the user back. Rejected navigation
does not retire an otherwise reconciled writer. Recovery reads that same turn,
never sends it again. A response lost before any server ID leaves an uncertain
writer; it does not search unrelated conversations or guess an ID.

Initial ownership still uses the committed composer host. Subsequent ownership
checks use memory, not send-button availability. This is not composer-free cold
startup, Android HTTP, WebView removal, or a measured thermal improvement.

## Offline Evidence And Next Acceptance

- `fresh-new-full-regression-20260913-124605-020`: 270 tests passed, zero failures
  or skips, including pinned source contracts, binding compatibility and shared
  stream regressions. The real module composition covers first send plus a
  second existing-conversation send, stop/partial text, project association,
  single ID adoption and SSE/v1. Its original history-before-navigation order
  was superseded by the September 13 first-route follow-up linked above.
- Failure cases cover missing server ID, wrong branch/root/privacy/identity,
  changed navigation entry, rejected/late navigation and duplicate native
  commands. Successfully applied history with failed navigation stays pending;
  no second POST or automatic send fallback occurs.
- The first navigation-failure test asked for recovery before the asynchronous
  stream had finished. It was corrected to await history entry; that fixture
  failure is not recorded as product or live acceptance.
- This source batch changes JavaScript only. It has not built or installed an
  APK, sent an account message or measured phone behavior.

Use `fresh_text_trial_start` for one current document/account/route command,
or explicit `__elonChatGptFreshTextNewConversationsEnabled === true`. Projects
and tools additionally require their existing admissions. No production flag
is silently changed by source tests.

Resolve the recorded parent mismatch before repeating native acceptance. From the actual
production native UI, start one controlled new ordinary conversation, send,
verify one server conversation and complete native reply, then follow up in the
same conversation. Accept a project root separately and restore prior state.
Temporary, attachments, tool combinations, no-assistant stop and real lost-ID
network recovery are not implied by either ordinary sample.
