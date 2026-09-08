# Official runtime message submission

Capability: `android_chatgpt_official_runtime_text_submit_v1`.
Status: implemented and installed, not live-accepted or completed. It is an
official page-runtime bridge, **not an independent Android HTTP/private POST
transport**. Reuse the existing bridge; do not reimplement it while resolving
the current production readiness failure.

Latest installed checkpoint: APK 1554 / adapter 299 includes runtime submit 11
and bindings 3. Two consecutive production native sends received exact test
replies and settled, but both still used DOM fallback: `submission_not_ready`
and then `conversation_route_mismatch`. The added generation-state reader has
not been independently observed on the device; settled workflows do not prove
which reader ran. See [1554 evidence](reports/chatgpt-runtime-release-1554.md).

## Current draft and guest ownership

APK 1555 contains submit 12 / bindings 4. Its first production current-draft
send reached the runtime without DOM fallback and later showed exactly one
complete test reply with native streaming false. However, the receipt reported
`unknown:context_changed`. The subsequent read-only check confirms delivery,
not which individual post-submit ownership check changed.

Submit 13 separates confirmed dispatch from editor/UI cleanup for current-draft
transactions. Only the official completion resolving exactly `true`, under
`requireDispatchAcceptance`, acknowledges the captured command. A reset editor
cannot revoke that result. A changed current context sets `current: false`:
the orchestrator does not start synthetic streaming in the successor page,
and the runtime never clears its draft. Android matches the original request ID
against its send ledger. False/malformed/failed completions remain unconfirmed;
explicit text and attachment cleanup retain their stricter original contract.
This extension awaits its grouped Release acceptance.

The bridge reuses the official runtime
writer rather than adding another HTTP sender. Known ready explicit text and
native attachment submissions are unchanged.

The retained September 7 composer source distinguishes explicit-action
readiness (`rc`) from `current_draft` eligibility (`So && tc`). Its shared props
expose `rc` as `isComposerSubmissionReady`; using that Boolean for both actions
over-restricts ordinary draft submission. The official transaction separately
checks composer constraints, mode, input validity and dispatch acceptance. We
do not modify any eligibility field or construct missing model/tool metadata.

When explicit-action readiness is false, the bridge may use the current-draft
contract only with the exact mounted official editor and no attachments. The
versioned facade maps these public exports, verified against both retained assets:

| Contract | September 6 symbol/export | September 7 symbol/export |
|---|---|---|
| Controller-owned editor | `CL` / `t_` | `XL` / `__` |
| Current document text | `kN` / `AS` | `pP` / `KS` |
| Replace plain editor text | `AN` / `VS` | `mP` / `rC` |

Only plain paragraph/text drafts qualify. The existing editor must belong to
the captured composer DOM and controller; no new controller/editor is created.
The official replacement transaction does not focus the editor and receives
`scrollIntoView: false`. A matching draft is not replaced. Document, identity,
thread, selected leaf, file store and draft are checked again before submitting
`current_draft` exactly once with `requireDispatchAcceptance`. The website owns
draft reset, proof creation and state updates. A failed local handoff, rejection
or ambiguous invocation is not replayed through another writer.

Draft comparison reads the current `view.state.doc` through the official pure
text converter. The UI subscription getter is intentionally not used: its editor
change notifications are delayed, so immediately rereading that computed value
could reject a transaction that has already updated the editor document.

The positive logged-out homepage can retain the same official conversation
after its server ID is allocated without changing the root URL. The bridge
allows this case only on the exact root URL with the existing double guest
proof and committed conversation/controller ownership. Non-UUID server IDs,
authenticated root mismatches, projects, temporary routes and other conversation
URLs still reject. An already assigned server ID cannot change during dispatch;
new-thread allocation still requires the same official object. The public shared
source `P$e` assigns `serverId$` and remaps its store to the same object.

The official composer module is warmed once through the existing bounded,
single-flight document cache. An unknown/cold module remains unavailable without
queuing a send. These source checks do not by themselves prove runtime acceptance
or a measured latency/thermal improvement.

## Resident structured-input host

Runtime submit 11 / global adapter 298 is installed in APK 1552. The retained
current public composer (SHA-256
`36644eb82aac9c399bce384c18140f8c878dd780c8f787440b80f27971729733`)
unconditionally calls `Iue` inside `uqn`, then passes the returned
`structuredInputHost` to shared props. Current conversation SHA-256
`7973d518b083f0f3e23905a279ed019378481bdbdd10fc0196afe9fc7b3b7d35`
exports that function as `AY` (`z9r`): it returns `canOpen$` and `tryOpen$` methods,
not an active-input flag. Rejecting every non-null host blocks ordinary chat.

The repair allows only this exact two-method host with an explicitly null
structured message ID. It does not call either method. Unknown host shapes,
active structured messages, missing mode proof, disabled submission and policy
checks still reject before any write. Captured host identity and thread mode are
revalidated before submission; the existing single-writer rule remains unchanged.

The production MCP receipt identifies the mode-gate failure, but the exact
subcondition was not live-inspected: this Release APK exposes no debug socket.
Temporary debug forwards were removed; no other browser was inspected or changed.
The source evidence establishes the overstrict guard. New synthetic cases failed
before the repair; 172 focused and then 259 integrated runner cases passed. The
installed send advanced to `submission_not_ready`, not an accepted transaction.
Current official `submitComposer` also enforces explicit-action readiness inside
its own transaction; deleting our guard alone is not a proven repair. The next
gate remains `official_runtime_v1:accepted`, one reply and settled native state.

## Current-tree membership

The prior resolver accepted a host or its alternate when its return-pointer
chain reached `root.current`. This both rejected valid reused children and could
admit stale children absent from the committed parent's child list. React's
[official tree reflection](https://github.com/facebook/react/blob/main/packages/react-reconciler/src/ReactFiberTreeReflection.js)
documents child reuse across parent alternates during bailout. The new resolver
is a bounded, read-only membership search, not a copied React implementation.
It considers each child's parent and parent alternate, verifies child/sibling
membership at every edge and requires exactly one path to the current root.
Only that path supplies context values; stale-branch props are not merged.

Submit 9 retained the nearest DOM host boundary and 90-fiber path limit, with
180 visits, 512 siblings per parent and 4096 sibling inspections. The 1550
phone subsequently hit its combined path/visit limit; this does not establish
the actual complete website depth or prove bailout caused the earlier failure.
Submit 10 raises the engineering budget to 512 path entries and 1024 visits,
leaving the DOM-host and sibling budgets unchanged. Memoized linked records
replace copied ancestor arrays: path storage grows linearly with visits and
only the final selected path is flattened. Depth and visit failures now have
separate fixed codes. Cycles and ambiguous membership remain rejected, without
polling, page reload, alternate writes or credential changes.

Eleven added cases failed on submit 8 and passed on submit 9; its integrated
run passed 236 Node cases. Three deep-tree cases (120/400/510 added ancestors)
fail on submit 9 and pass on submit 10, asserting one child-list read per parent;
the over-budget case still rejects. Submit 10's integrated run passes 239 cases
across bindings, text/attachments, stop, regeneration, lifecycle and receipts.
This is not live runtime acceptance or a measured device latency/thermal gain.
The next grouped APK must obtain an accepted runtime receipt plus one reply.

## Confirmed guest transactions

Missing request headers are not an authenticated identity, but neither do they
prove the official guest composer cannot send. Runtime submit 8 allows its text
transaction to use a positively recognized guest context: the official bootstrap
must report exactly `logged_out` and the live session getter must return exactly
`null`. Undefined, loading, throwing, missing or conflicting state is not accepted.
The authenticated path still needs valid captured Bearer headers and imports no
additional module. The shared `captureConversation` API remains authenticated by
default, so this extension does not silently widen stopping or regeneration.

The existing versioned binding cache now exposes a synchronous `peek`. The text
module warms one recognized shared module asynchronously; imports retain the
existing single-flight, 1.5-second timeout and 10-second failure cooldown. A cold
send remains available to the existing path immediately. Warming never queues,
retries or replays a user command. Subsequent sends use the frozen current-page
namespace as guest identity and recheck both official getters before and after
dispatch. Login, changed headers, document/token replacement or mixed runtime
builds invalidate ownership and cannot clear another context's draft.

The public shared assets were hash-checked on 2026-09-08:

- Legacy `4813494d-hrplraurzfyvxb10.js`, SHA-256
  `89c95d937bac1191e91d5ceb4872eb0c328d39a98ce05399093a663f18921aa0`:
  exported `R5` is bootstrap getter `Pn`; exported `F5` is live session getter `In`.
- Current `4813494d-o593jrji51wy4azk.js`, SHA-256
  `48563cd22f0dafe6c0b89220348fa3add81ff3abb82a62ed9d68a04d569cc375`:
  the corresponding exports are `i7` and `t7`. Import aliases inside these files
  are not their public export names. `In` returns the official store's current
  session; bootstrap text alone is not used as live identity proof.

Verification: **222 targeted Node tests passed**, zero failures/cancellations/
skips, across bindings, text submit, attachments, transaction lifecycle, send
settling/observation, stop, regeneration and current-runtime consumers. New guest
acceptance/cold-cache/ownership cases failed on runtime submit 7 before the fix.
This proves source integration, not a live guest writer or latency improvement.
The next grouped APK must produce `official_runtime_v1:accepted` plus one actual
reply in the production native UI; the previous DOM result is not reused as proof.

## Scope and ownership

The native send transaction can invoke the current official `submitComposer`
with `text_action` and `requireDispatchAcceptance`. This removes the ordinary
fill-editor, wait-for-send-button and click sequence when the recognized runtime
is already ready. Fresh proof preparation, model/tool policy, conversation state
and generation remain owned by the website. Existing stream observation and the
native send ledger are reused; voice, dictation and Google are unchanged.

Runtime submit 5 exposes the same committed conversation context for
[official runtime stopping](chatgpt-official-runtime-stop.md), without requiring
the composer to be input-ready. Submit and regeneration remain blocked while a
stop request is in flight; this is not another sender or a new identity layer.

The bridge requires the inspected module to be present, one committed composer
context, one file store, a matching conversation/controller, and known readiness
and policy fields. It binds the document, identity, route and selected leaf before
invocation. Credentials remain page-local and never appear in receipts. Unknown
contexts, structured input and pending or unowned attachments leave the existing
sender available **before any runtime write**. The candidate admits
ordinary conversations and the currently selected new project/temporary context;
other routes are not guessed.

After invocation, there is exactly one writer. Synchronous `accepted: false` is
an unsent rejection. Acceptance requires the official completion to resolve to
`true` while ownership still matches. Errors, ambiguous completion and a 15-second
observation timeout are indeterminate, never a reason to send again by another
route. The bridge retains its in-flight ownership after timeout until the
underlying completion settles. An invocation exception or malformed receipt
retains ownership until document replacement; there is no automatic reload or
write replay. Recovery for such a protocol mismatch remains an acceptance item.

Explicit text/prepared actions do not reset the official editor. Only an unchanged, nonempty
draft matching the submitted text is cleared after acceptance; subsequent user
edits survive. The existing native receipt policy records `OFFICIAL_PAGE`, not
`SAME_ORIGIN_PRIVATE`, including indeterminate receipts. A successful receipt
does not prove that the response finished or that first-token latency improved.

## Native attachment handoff

The 2026-09-07 extension reuses the private upload sender's existing composer
owner. The initial composer 15/sender 14 lease admitted exactly one native-owned
ready file. Composer 16, sender 17 and runtime submit 4 now admit the exact
ordered batch of up to nine owned ready entries in one `prepared_action`;
see [native batch ownership and checks](chatgpt-private-attachment-batch.md).
The existing orchestrator and adapter wiring are reused.
This removes DOM fill/button polling for recognized ready attachments; it is
still official-runtime submission, not independent private HTTP generation.

Preparation requires the exact file-store object and current account, document,
route, model/library policy and project branch. Other ready entries or an active
upload are not admitted. A detached, frozen metadata snapshot retains the exact
File reference plus file spec, image dimensions, source, library IDs and project
metadata. Identity and a metadata fingerprint are rechecked before dispatch;
matching names or IDs cannot substitute another file. No upload or send occurs
while preparing the lease.

Android attachment reservations still pass `privateTextTransactionAllowed=false`.
This now permits only the runtime's native-attachment contract; it does not enable
the pure-text captured-request relay. Without a native attachment, that flag
retains its previous behavior. Older runtime instances cannot ignore this new
restriction, and an in-flight older writer survives adapter reinjection.

After official acceptance and conversation-owner confirmation, cleanup removes
only the submitted objects. A new thread may already have acquired its server URL,
and a display snapshot may have released its previous URL-bound owner. Later
user files/drafts are preserved; a user removing the submitted file in flight
does not turn acceptance into an error. A failed local store cleanup reports
`attachment_cleanup_unconfirmed` and retains writer ownership, preventing another
send with an already accepted attachment. This exceptional retained state, like
malformed runtime receipts, still needs explicit recovery acceptance; it does
not trigger a reload, second upload, file deletion or alternate send.

## Source evidence

Inspected public assets, retrieved without account credentials:

- [Composer runtime](https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js),
  SHA-256 `9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5`.
- [Conversation runtime](https://chatgpt.com/cdn/assets/conversation-small-hiw4wce20lu6te81.js),
  SHA-256 `296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1`.

In the first asset, `j$n`'s `xu` gates eligibility and returns
`{accepted, completion}`; `Su` is passed as `submitComposer` into `nOn`. `UDn`
writes those props into the shared context store. Existing official consumers
call this exact `text_action` transaction with `requireDispatchAcceptance`.
`TOn`/`EOn` assembles the live generation state and awaits dispatch acceptance;
only the current-draft branch resets the editor. The second asset defines the
shared store's `getSharedProps` and subscription contract. These are public
source observations, not a captured successful authenticated request.

The same pinned composer asset contains an official consumer that filters its
ready-file entries and invokes `submitComposer` with
`{kind: 'prepared_action', text, readyFiles}` and `requireDispatchAcceptance`.
`EOn` passes these entries to `UVe` rather than reading the editor's pending files.
Its imported `Vdt` is the conversation asset's `Gar`, also used by ordinary draft
formatter `War`. It preserves attachment IDs, library/project attributes and
image asset pointers while assembling content for the current model/tool state.
The prepared branch does not reset the file store; its caller owns cleanup.
The artifact SHA-256 values above were rechecked for this extension.

## Verification and next acceptance

On 2026-09-07 the focused Node command passed **60 tests**, with zero failures or
skips, across `test-chatgpt-web-text-runtime-submit.js`,
`test-chatgpt-web-private-text-transaction.js`,
`test-chatgpt-web-text-transaction-lifecycle.js`,
`test-chatgpt-web-send-settle.js`, and
`test-chatgpt-web-private-send-observer.js`. It covers current React ownership,
context/draft changes, unowned attachment exclusion, new conversation IDs, uncertainty,
single dispatch, production orchestration and asset-bundle syntax.

After the attachment extension, **196 focused Node cases passed**, with zero
failures, cancellations or skips. This includes 25 new submission-lease tests and
28 new runtime-attachment tests, the original runtime/text/send suites, composer,
library integration, project/thread and image suites. Synthetic production-module
integration verifies ordinary/temporary/image/reused ready entries, exact dispatch
arguments, unchanged private-replay prohibition, URL allocation, later drafts/files,
stale owners, timeout/rejection/malformed receipts, cleanup failure and reinjection.
Project upload tests are retained; real project prepared dispatch is not yet proved.

The initial source-only round did not build an APK. Grouped APKs 1547 and 1548
were subsequently built, published and installed, but their text samples fell
back to DOM. Adapter 294 in 1548 includes the owner correction and regression
tests; the runtime submit 8 guest extension above still awaits the next grouped
build. Neither installed fallback result accepts the direct runtime writer.

In the grouped production UI round, confirm that the actual committed context
resolves, send one synthetic text, retain the `official_runtime_v1` receipt and
stream result, and check draft preservation and conversation ownership. Check
the admitted project/temporary contexts and the native picker/upload/ready-entry
handoff without claiming unsupported scopes. Verify no extra upload, duplicate
message or lost attachment, and confirm the response actually references the file.
Multi-file native leases are implemented as described above, but live prepared
dispatch remains unaccepted; existing paths remain available before a runtime write.
Independent fresh-proof private HTTP dispatch, private regeneration, measured
latency and resource improvement are still separate unfinished work.
