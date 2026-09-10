# Library Attachment Selection Refresh

Scope: a file still visible in the native Library browser remains actionable
after the directory cache's 60-second freshness window. This extends the existing
catalog and attachment owner, not the upload transport or file mutation authority.

## Status

- Capability: `android_chatgpt_private_library_selection_refresh_v1`.
- Implementation: catalog v8, attachment v7, sender v25, adapter 320; default
  native path. Commit `98b0f7b61` additionally compares structured metadata by
  value and preserves the closed scope-rejection receipt.
- Offline verification: 174 related checks pass in
  `library-metadata-scope-related-20260910-171229-781`, with no skips/cancellations.
- Delivery: normal 1.1.1637 / adapter 321 includes the grouped correction and was
  published/installed over wireless ADB with login preserved.
- Device verification: completed for ordinary visible TXT association after a
  67-second wait with foreground/context checkpoints. The actual Attach button
  succeeded; the reference was subsequently consumed in one mixed attachment
  send. See [1637 evidence and exact boundaries](reports/chatgpt-library-local-append-1637.md).
- Previously accepted ordinary append/send on 1635 remains reusable; this change
  does not claim mounted materialization or reverse-order local picker acceptance.

## Cause And Contract

The browser retained a visible file while `selectAttachment` delegated directly
to the 60-second `selectMutation` ticket. Clicking later failed before making any
request. Extending every ticket would also weaken rename/delete safeguards.

Each catalog item now keeps its actual observed list-page URL and observation
time inside the page owner. Neither is exported in native snapshots. A fresh
selection requires no request. An aged selection reuses that exact same-origin
GET `/backend-api/files/library/nodes`, including parent/query/cursor parameters.
No unobserved per-file endpoint is invented and no DOM click or page reload runs.

The bounded response must contain exactly one matching source ID. Its attachment
identity, name, MIME, size, parent and source/scope fields must still match. Missing,
duplicate or changed records are not attached; network failures remain unconfirmed
operations, not claims that the official product lacks the feature. Unrelated
server access/update timestamps do not invalidate an otherwise identical file.

Structured fields are compared recursively, not by JavaScript object identity.
A new JSON response necessarily has new array/object references. Object key order
is irrelevant; array order, missing/additional fields, types and values still
must match. Comparison is bounded to depth 32 and 4096 visited values across the
selected attachment fields. Exceeding either bound leaves the selection
unconfirmed. The old reference-equality check incorrectly rejected identical
structured metadata; the new JSON-roundtrip regression fails on adapter 319
(`library-metadata-structure-red-20260910-171147-783`). This is a reproduced code
defect, not a claim that it caused any particular interrupted device attempt.

Identity, account, document, route, transport and selected-item ownership are
checked before and after the read. Replaced/evicted rows, local mutations,
cancellation and late responses cannot revive the old selection. Only this
item's attachment freshness is renewed, not siblings, list freshness or mutation
tickets. Cache capacity remains eight pages / 500 items per page.

The GET retains the existing 10-second / 2 MiB request bound and consumes the
attachment owner's existing 24-second whole-operation budget. Identical request
IDs join its existing receipt; no duplicate association or remote write replay is
introduced. Once validated at click time, the selected file remains usable for
that bounded operation even when the display TTL passes during scope preparation.
The account, selected row and composer still must remain unchanged.

## Focused Evidence

`scripts/test-chatgpt-web-library-selection-refresh.js` first fails on the original
code (`library-selection-red-20260910-161827-091`). It covers fresh zero-extra-read
use, aged native association, per-item renewal, actual parent/query/cursor reuse,
snapshot privacy, metadata/scope drift, identity/owner replacement, request errors,
single-flight, cancellation, deadline and the 59-second click boundary. Related
catalog, mutations, append, mounted preparation, bridge and native wiring tests
remain green. Device acceptance must select one fixed existing fixture through
the production Library UI, wait beyond TTL, attach once, confirm the ready card,
remove it and restore the prior conversation without sending a message.

## Normal 1636 Attempts

- Release source `089795b08`; APK SHA-256
  `6582a4743f36a35f1ad21163ca243983b2a31dce333e59fd04524908accb83fd`.
  `library-selection-release-20260910-162900-651` built, published and installed
  normally. This package does not contain the later structured-metadata fix.
- `library-selection-native-20260910-163755-654` stopped before clicking because
  another application's Activity was foreground. This is not private API failure.
- `library-selection-native-resumed-20260910-164342-983` clicked Attach and returned
  `composer_context_unavailable`. Subsequent structural inspection showed that
  the conversation was a project route, not the accepted ordinary scope. The
  installed catch also collapsed `library_attachment_scope_unconfirmed` into
  that generic error, so it cannot prove a missing input/store. The new closed
  error mapping is tested without exporting exceptions or weakening scope guards
  (`library-scope-receipt-red-20260910-170550-804`).
- Ordinary-scope attempts `library-selection-ordinary-native-20260910-165705-362`
  and `library-selection-native-resume-20260910-170509-627` did not establish an
  attachment result. The former stopped before an attachment receipt; another
  app was subsequently foreground. The latter detected selected-view changes
  after 65 seconds and stopped before clicking. Exact changed fields were not
  captured, so do not infer another composer or server failure from it. Both
  restored the original conversation and verified unchanged native messages;
  no message was sent and the final draft/attachment state was empty.

The later 1637 acceptance bound the ordinary route and checked the foreground and
context throughout the wait before using the actual Attach button successfully.
The interrupted 1636 attempts remain historical failures, not outstanding repeats
of this now-accepted ordinary scope. Project/temporary and mounted association
remain separate; no new endpoint or scope permission is inferred from local upload.
