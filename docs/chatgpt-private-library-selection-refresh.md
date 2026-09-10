# Library Attachment Selection Refresh

Scope: a file still visible in the native Library browser remains actionable
after the directory cache's 60-second freshness window. This extends the existing
catalog and attachment owner, not the upload transport or file mutation authority.

## Status

- Capability: `android_chatgpt_private_library_selection_refresh_v1`.
- Implementation: catalog v7, attachment v6, adapter 319; default native path.
- Offline verification: 163 related checks pass in
  `library-selection-related-20260910-162109-451`, with no skips/cancellations.
- Device verification: pending the grouped normal Release containing this change.
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
