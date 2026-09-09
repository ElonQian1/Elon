# Private conversation shared-link management

Capability: `android_chatgpt_private_conversation_shared_links_v1`.
Current source: shared-link module 3 and share owner 5 add account-wide personal
link browsing through the existing production Share menu. It is not installed
or device-accepted yet. Earlier shared-link module 2 was installed in APK 1545. The production
private list returned a complete empty result for the unshared synthetic fixture
in 1,436 ms, replacing 1544's early `share_scope_unconfirmed` failure (54 ms).
In `1.1.1547`, case
`android_chatgpt_private_conversation_shared_links_v1:personal_list_revoke`
is completed through the production handler: the newly created synthetic link
appeared in a complete list, its exact selection ticket was consumed, and revoke
returned confirmed removal. See [device evidence](reports/chatgpt-runtime-release-1547.md).
Rendered native menu/Copy and other account scopes remain pending; these narrow
completed cases do not claim every sharing variant is complete.

## Official contract evidence

The current settings module `c2675c8c-f6cd0ubcb7y7eluj.js` routes its
`manage-shared-conversations-button` to the lazy module below. The exact public
asset was downloaded on 2026-09-07, not guessed from an endpoint convention:

`https://chatgpt.com/cdn/assets/04d63960-ce9ew1r7qtx0jund.js`

SHA-256: `9d979e04d7d00ee11297a7f9ea7446e6aa0b6d01429be7c1238a884893dbe230`.

- `W` calls `H1.safeGet('/shared_conversations', query: {order: 'created'})`.
  The UI consumes `items`, `total`, each row's `id`, `conversation_id`, `title`,
  `create_time`, and nullable `workspace_id`.
- Row `J` calls `H1.safeDelete('/share/{shared_conversation_id}')` with the
  selected row ID. It invalidates the shared-conversation query afterwards.
  It does not delete the source conversation.
- Personal links use `/share/{id}`; workspace links use `/share/e/{id}`.
- The same manager separately loads/deletes Canvas textdocs and posts. Its
  bulk-delete menu also covers those resources. Those are not aliases for
  individual conversation-link revocation and are not implemented by this batch.

The app's existing same-origin transport supplies the `/backend-api` prefix.
These observations establish the published caller contract, not authenticated
request/response success. The in-app browser navigation attempt timed out; it
provided no live evidence. No real share link or membership was changed.

## Production behavior

The existing production conversation action -> Share coordinator offers
creation/member sharing, management of that conversation's existing public
links, and management of all personal public links. It does not create a second test page. Management can run without
navigating away from the current thread or touching its draft/recording state.

`WebChatConsumerPort.manageConversationShares` uses the existing
`chatgpt_share_conversation` action and tracked `share_conversation` receipt:

- `operation: list` and `conversation_path` request a read only.
- `WebChatConsumerPort.manageAccountShares` uses `operation: list_account`, an
  optional `page_offset` and `selection_ticket` on the same tracked command.
  It rejects a supplied conversation path or share ID and does not publish.
- `operation: revoke`, `conversation_path`, `share_id`, `selection_ticket`, and
  `user_confirmed: true` request one explicitly selected cancellation.
- Unknown operations, arbitrary URLs/IDs and unconfirmed revocation are rejected.
- New public sharing keeps its existing explicit confirmation and runtime-bound
  creation flow. Project-member sharing remains a separate audience; its URL
  does not become a deletable public share or revoke project membership.

The private module reuses the existing share transport and page-local account
identity. List/revoke bind to that identity and server-returned personal-link
rows, not to loaded conversation modules. Publication retains its separate
version-pinned runtime/branch contract. Each read/write is bounded to seven seconds,
with a one-MiB response limit. Only approved same-origin headers are used; no
copied proof token, Cookie export, or raw response logging is added.

One in-memory, document/account-bound list cache lasts 60 seconds. List results
expose at most 100 links for the selected conversation, plus an explicit
`complete` flag. A server total larger than the returned rows remains partial;
no unobserved pagination parameters are invented, and missing/invalid response
data cannot produce an empty-success result. Workspace rows are not offered in
this personal-link flow; a selected conversation with unsupported workspace rows
remains partial, not falsely complete and empty. No background polling or disk
credential cache.

Account browsing pages that same already-returned snapshot, 100 personal links
at a time (at most 1,000 parsed rows). Its `elon.account_shares.v1` receipt carries
each link's source path, offset and next offset. Next/previous reuse the exact
selection ticket for up to 120 seconds, without another HTTP request or mixing
pages from different snapshots. Expired/foreign tickets and invalid offsets
fail before network access. Workspace rows and server-partial collections stay
explicitly partial; this is not server-pagination support. Native labels reuse
the cached conversation title when present, otherwise a numbered link and date.
No extra conversation fetch is needed just to label the list.

Cancellation requires an unexpired (120-second), account/document-bound selection
ticket and exact source-conversation/link match. The ticket is consumed before
DELETE. A fresh list must completely exclude the link before success is shown.
A timeout, partial readback or still-present link reports an uncertain result;
no automatic second DELETE occurs. The creation-result cache is invalidated
before cancellation so subsequent Share cannot return the locally cached old
URL. A successful readback updates the list cache immediately.
Publication also invalidates the list before its first write, even if the
response later times out. Read-only reconciliation stays available during the
write cooldown, so an old empty cache cannot hide a possibly created link.

Native UI supports link selection, Copy, explicit cancellation confirmation,
return to list, and manual official-page recovery. It distinguishes complete
empty lists, partial results and failed reads. A structured receipt validator
allows only bounded IDs, timestamps and selection metadata; private titles,
headers and arbitrary URLs cannot enter the command ledger through this result.

## Verification and remaining work

### Account browsing extension, 2026-09-10

- Five new account scenarios first failed against the unchanged owner, then
  passed with the extension. All **159 related Node runner cases** pass.
  Coverage includes cross-conversation listing/revocation, 205-row fixed-snapshot
  paging, read-only reconciliation during write cooldown, stale identity and
  partial/workspace rows. HTTP responses are synthetic, not live acceptance.
- Actual Kotlin 2.0.21 compilation and JUnit: **7 policy tests passed**, including
  strict account receipt validation, source path retention through the real
  command sanitizer and invalid-page rejection. A subsequent grouped Gradle
  `:app:testDebugUnitTest` compiled production Kotlin/Java and test sources, then
  passed **11 tests** in `ChatGptWebSharedLinksTest` and
  `ChatGptWebConversationShareMcpTest` (zero failures/errors/skips). This includes
  the native consumer path with no ready composer, unchanged draft, no navigation
  and rejection of invalid origin/offset/scope. No installable Debug APK was built.
- Reuses the September 7 official caller evidence above. A fresh public-asset
  fetch in this batch failed with a connection EOF; no new source hash or
  authenticated account response is claimed. No new endpoint was inferred.
- The new account UI/page controls and a specifically selected cross-conversation
  fixture revoke still require the grouped install/production acceptance.

### Earlier scoped delivery checks

- 1545 real-device case
  `android_chatgpt_private_conversation_shared_links_v1:list_personal_empty`:
  completed through the production handler/MCP. `complete=true`, zero links for
  the unshared synthetic conversation, no publication/revocation or new message.
  Restored `conversation_home` with draft length zero. Reuse this narrow evidence;
  non-empty lists, actual cancellation and UI selection still need acceptance.

- 2026-09-07 correction: 179 focused Node cases passed, including unloaded
  conversation modules, missing composer, server-bound revocation, stale identity,
  explicit confirmation and unchanged publication guards. The live 1544 failure
  is not presented as proof of a specific missing asset or account type; it only
  confirmed the early runtime scope gate blocked the read. Official manager `J`
  deletes the selected server-returned row without that conversation-module gate.

- New module absence reproduced the missing feature in the initial Node run.
- Final targeted Node run: **152 passed**, covering shared links, creation,
  member links, deletion and metadata mutation. Includes cache reuse/expiry,
  account/document change, partial/malformed responses, ticket binding, exact
  DELETE/readback, ambiguous outcomes, no repeated writes, creation-cache
  invalidation, uncertain-publication cache reconciliation and concatenated
  production asset syntax/order.
- Actual Kotlin 2.0.21 compilation and JUnit: **13 passed**, covering the new
  structured result/request policy plus existing public/member URL receipts and
  share policies. The first compile command omitted the existing protocol helper;
  adding that source resolved the harness dependency. No product compiler error
  was hidden or waived.
- Native consumer/MCP routing test added for list-without-publication, explicit
  revoke confirmation and no navigation/draft mutation. This broader Android
  test and the two UI coordinators await grouped Android compilation/execution.
- No new APK, screenshot/visual acceptance, authenticated cancellation, or
  performance/thermal measurement was performed in this source batch.

Next acceptance: from the real social-AI production Share action, list an
authorized synthetic conversation's existing public link, copy it, cancel only
that test link with confirmation, verify that the original conversation remains
and the link no longer grants access. Check the draft/current thread stayed
unchanged and the list updates. Do not publish or revoke real personal content
merely to test this module.

Remaining distinct scopes: account-wide UI device acceptance, workspace links, bulk
revocation, Canvas/post/task shares and full-list pagination when the server
returns a partial collection. They must not be reported as completed by this
implementation. The account-wide personal UI is implemented in source; workspace,
Canvas and post links are not silently treated as personal conversation links.
