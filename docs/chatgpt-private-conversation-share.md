# Private full-conversation sharing

Capability: `android_chatgpt_private_conversation_share_v1`.
Status: implemented for the standard authenticated personal-account,
non-project conversation flow, including legacy and redesigned variants;
offline verified and shipped in `1.1.1547`. Case
`android_chatgpt_private_conversation_share_v1:personal_create` is completed:
production handler created a link from a synthetic conversation, its private list
returned that exact link, and revocation succeeded. This does not accept every
website variant or the rendered native confirmation/Copy/share-sheet interaction.
The accepted release used adapter 293, share contract 3 and transaction module 4; see
[current device evidence](reports/chatgpt-runtime-release-1547.md). The separate
[project-member resolver](chatgpt-private-project-conversation-share.md) now
reuses this command owner with distinct member-only consent and receipts;
it does not use the public writer described here.

## Composer-independent readiness

Current source uses share contract 4 and transaction module 5. The 2026-09-10
readiness correction is offline verified and published/installed in normal
`1.1.1630`; native UI acceptance remains pending. See the
[grouped delivery](reports/chatgpt-native-regeneration-20260910.md#normal-1630-grouped-delivery).

Native `ACCOUNT_MUTATION` admission and the share coordinator already allow a
loaded conversation without its composer. The private contract redundantly
required `composerReady`, blocking public sharing and project-member links;
composer unmount during a request also discarded an otherwise valid result.
The official modal asset referenced below was rechecked on 2026-09-10 with the same hash:
its writer uses the selected conversation/node, not input-element readiness.

Contract 4 removes only this composer dependency. It retains exact snapshot URL,
document/account/branch/variant ownership, loaded runtime conversation, explicit
non-streaming state, dictation exclusion, native confirmation, moderation checks
and no replay of uncertain writes. Draft and attachment state are unchanged.
Member resolution still requires current project membership and never creates
a public link. The existing 60-second caches survive composer-only changes.

The two share suites reproduced 12 failures before the correction (128 tests,
116 passes). Afterward, those suites plus shared-link management pass 158/158,
with no skipped/cancelled tests. Added cases cover missing/unmounted composer,
cached reuse, draft preservation and unknown streaming state. Existing
account/document/branch drift, consent, project scope and uncertain-write tests
remain green. No sharing write was made on a real account for this source batch.

## Official evidence

The public modal loader
`https://chatgpt.com/cdn/assets/c2675c8c-f6cd0ubcb7y7eluj.js`
loads `https://chatgpt.com/cdn/assets/c2547acf-jpdhb2pddhaqyu5p.js` for
`sharingModalThreadId`. The latter asset, inspected 2026-09-07, has SHA-256
`f806bd3353bd04fb3410dc077418a632d02ec4ebbefe1c6d79988f1d48d9f4f7`.
Its `gt` creator and `St` publisher establish this full-conversation contract:

1. Resolve the selected completed message with the conversation module's
   `AGt` (`H9t`), not the last server message or a sidebar title.
2. The legacy authenticated modal POSTs `/share/create` with
   `current_node_id`, `conversation_id`, and `is_anonymous: true`.
3. The response supplies `share_id`, `share_url`, visibility, anonymity,
   current node, title, optional highlighted message, and moderation state.
4. Before distribution, PATCH `/share/{shared_conversation_id}` with
   `highlighted_message_id`, `title`, `is_public`, `is_visible`, `is_anonymous`,
   and the selected `current_node_id`. An existing link may reference an older
   node; the PATCH updates it to the selected branch.
5. A successful publisher response includes `moderation_state`. Its `yt`
   predicate rejects `has_been_auto_blocked`, `has_been_auto_moderated`, or
   `has_been_blocked` when true. An explicit empty object is non-blocked;
   missing/malformed moderation data is not accepted by our adapter.

The API client prefixes these routes with `/backend-api`. Shared runtime
`4813494d-hrplraurzfyvxb10.js` has SHA-256
`89c95d937bac1191e91d5ceb4872eb0c328d39a98ce05399093a663f18921aa0`.
`H3` tests authenticated bootstrap state. `mq` returns the account model;
`wV(SV.isPersonalWorkspace)` checks personal workspace. `XM` and `HM` expose
the loaded conversation and branch selectors. The conversation asset
`conversation-small-hiw4wce20lu6te81.js` has SHA-256
`296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1`.
Its `Gkt` separately gates project, temporary, health, owner and workspace flows.

These are downloaded official-source observations, not captured authenticated
POST responses. No actual share link was created during this offline batch.

## Redesigned create transaction

The same official modal's `gt` chooses `/share/v2/create` for `modal_redesigned`
and `toast` variants (and separately for unauthenticated visitors). `St` omits
its `beforeShareAction` legacy publisher for those branches. Accordingly, the
authenticated redesigned path sends one POST with the same conversation/node/
anonymous body; it must not automatically issue the old publish PATCH afterward.

The inspected conversation module exports `Ojt` as `J5t`. It reads experiment
`2483695150`'s `dweb_conversation_share_sheet_variant`, yielding `control`,
`modal_redesigned` or `toast`. The composer asset's `Lnn` current-conversation
Share action uses this getter; `Inn` also calls it with `disableExposureLog: true`.
The native coordinator already opens the selected conversation before sharing,
so its protocol selection uses that current-conversation getter, not the separate
sidebar override (`Ajt`/`Y5t`). It does not choose a protocol from cached UI labels.
These asset hashes were rechecked on 2026-09-07 and match the evidence above.

Variant is part of the transaction and 60-second cache identity. Missing exports,
unknown values or a changed variant reject the binding before writing. A change
during a request produces an unconfirmed result, never a second request through
the other writer. The v2 response must validate the canonical public link,
visibility, anonymity, selected node and nonblocked moderation before native
distribution. A missing response node follows the official requested-node
fallback; an explicit older node is not silently published or patched.
Actual v2 response fields/moderation and current runtime export binding remain
mandatory live acceptance items, not facts proved by synthetic tests.

## Production path

The production conversation action sheet now has a dedicated Share action.
It opens the selected conversation through the existing tracked navigation,
preserves drafts, asks for explicit public-link confirmation, then calls
`WebChatConsumerPort.shareConversation` -> `chatgpt_share_conversation` ->
the existing command ledger -> `share_conversation` -> page-local private
transport. It does not open a second test UI or click a DOM Share button.

`chatgpt_web_private_conversation_share_contract.js` owns version-pinned
runtime bindings and response validation. The companion share module owns
the create/publish transaction. The ordinary result is a validated canonical
`https://chatgpt.com/share/<uuid>` URL. Only a succeeded matching command
receipt enables native Copy or Android distribution of this official URL.
The system chooser does not create the official link or replace the protocol.

Safety and lifecycle behavior:

- Require native confirmation before any sharing write.
- Bind document, route, account, selected leaf/message, and title; reject
  loading, streaming, dictation, project, temporary, health and business scopes.
- Capture the website's current sharing variant and retain it through creation,
  optional legacy publication and result-cache validation.
- Reuse only the inspected modules already loaded by the current website.
  Missing runtime is unconfirmed context, not evidence the feature is absent.
- Keep identity headers page-local; never replay proof headers or export
  Cookie/authorization into native results or diagnostics.
- Serialize share/delete/metadata mutations; each HTTP request has a 7-second
  whole-response deadline and 256 KiB limit. Do not retry a write automatically.
- Uncertain writes do not expose a URL, trigger another endpoint, or navigate
  automatically to a potentially duplicate writer. Apply a 45-second cooldown
  and offer explicit official inspection.
- Reuse a successful link for at most 60 seconds only while its account,
  document, route, title and selected branch remain unchanged.

## Verification and remaining work

The earlier focused Node run passed **88 tests** across sharing, deletion and metadata
mutation. It covers exact bodies, old-node updates, concurrency, account/route/
branch drift, malformed/public URL rejection, moderation, timeouts without
replay, draft preservation, cache invalidation and the production asset bundle.
The 19 new variant cases all fail against the preceding implementation while
the existing 50 share cases pass. After the fix the combined 88-case run passes,
including legacy and both redesigned branches, no duplicate PATCH, variant/cache
drift, unconfirmed responses and no alternate writer after errors. Native policy
and actual consumer-to-MCP command tests are added but have not
been compiled or executed in this source batch. No APK was built or installed.

Grouped acceptance must use a synthetic ordinary conversation and explicit
public-share confirmation. Check the official result, native Copy/share,
updates after another message, failure state and preserved draft. Check both
officially selected protocol variants without forcing account experiment flags.
Verify exact
runtime exports and response shape on the phone before marking completed.

The subsequent member-sharing batch passes 131 combined Node cases and nine
pure Kotlin policy/receipt cases. The actual URL comparison bug and 160-character
member receipt truncation were reproduced and corrected. Native consumer/MCP
tests and full Android compilation remain pending; no real link was created.

The redesigned authenticated route is now implemented; guest `/share/v2/create`
remains a distinct unsupported scope. Personal shared-project member links are
source-implemented separately. [Conversation-scoped public-link management and
revocation](chatgpt-private-shared-links.md) are also source-implemented, with
grouped Android acceptance pending. Still separate code gaps: workspace and
private-project public sharing, eligible temporary-chat sharing,
bulk revocation and Canvas/post shares. Account-wide management and cached paging
are implemented; their current acceptance boundary is recorded in the linked
shared-links document. Do not reuse this public confirmation for a members-only link,
or mistake `/share/post` message-slice creation for full-conversation sharing.
Keep these gaps in the [remaining batch](web-ai-private-native-remaining-batch.md).
