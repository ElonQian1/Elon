# Official runtime message submission

Capability: `android_chatgpt_official_runtime_text_submit_v1`.
Status: implemented source candidate, not live-accepted or completed. It is an
official page-runtime bridge, **not an independent Android HTTP/private POST
transport**. Include it in the grouped ChatGPT APK; do not reimplement it while
waiting for production acceptance.

Latest installed checkpoint: APK 1549 / adapter 294 includes runtime submit 8
and bindings 2. A real guest reply still used DOM fallback. The identity gate
passed; the receipt now identifies `runtime_fallback:react_owner_unavailable`.
See [the release and production evidence](reports/chatgpt-runtime-release-1549.md).
Runtime submit 9 repairs a reproduced current-tree ownership defect below;
it is not yet installed or device accepted.

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

The nearest DOM host boundary and 90-fiber path limit are retained. Memoized
paths avoid exponential alternate traversal. Visits are capped at 180 fibers,
512 siblings per parent and 4096 sibling inspections per resolution. Cycles,
ambiguous ownership and either limit fail closed with fixed structural reason
codes. Pre/post-dispatch checks use the same resolver and preserve changed drafts;
no polling, page reload, alternate writer or credential change is introduced.

Eleven added cases failed on submit 8 and pass on submit 9. The integrated run
passed 236 Node cases, including bindings, text/attachment dispatch, stopping,
regeneration, lifecycle and fallback receipts. This proves a source defect and
its fix, not that the uninstrumented 1549 phone failure was necessarily caused
by bailout. The next installed probe must distinguish current-tree failure from
depth/child limits and then obtain an accepted runtime receipt plus one reply.

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
