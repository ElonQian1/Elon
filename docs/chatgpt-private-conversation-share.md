# Private full-conversation sharing

Capability: `android_chatgpt_private_conversation_share_v1`.
Status: implemented for the standard authenticated personal-account,
non-project conversation flow; offline verified; grouped Android compilation
and production-phone acceptance pending. This is not a completed capability
or a claim that all website sharing variants are implemented.

## Official evidence

The public modal loader
`https://chatgpt.com/cdn/assets/c2675c8c-f6cd0ubcb7y7eluj.js`
loads `https://chatgpt.com/cdn/assets/c2547acf-jpdhb2pddhaqyu5p.js` for
`sharingModalThreadId`. The latter asset, inspected 2026-09-07, has SHA-256
`f806bd3353bd04fb3410dc077418a632d02ec4ebbefe1c6d79988f1d48d9f4f7`.
Its `gt` creator and `St` publisher establish this full-conversation contract:

1. Resolve the selected completed message with the conversation module's
   `AGt` (`H9t`), not the last server message or a sidebar title.
2. The standard authenticated modal POSTs `/share/create` with
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

The focused Node run passed **69 tests** across sharing, deletion and metadata
mutation. It covers exact bodies, old-node updates, concurrency, account/route/
branch drift, malformed/public URL rejection, moderation, timeouts without
replay, draft preservation, cache invalidation and the production asset bundle.
Native policy and actual consumer-to-MCP command tests are added but have not
been compiled or executed in this source batch. No APK was built or installed.

Grouped acceptance must use a synthetic ordinary conversation and explicit
public-share confirmation. Check the official result, native Copy/share,
updates after another message, failure state and preserved draft. Verify exact
runtime exports and response shape on the phone before marking completed.

Still separate code gaps: redesigned/guest `/share/v2/create`, workspace and
project-member sharing, eligible temporary-chat sharing, share management and
link revocation. Do not reuse this public confirmation for a members-only link,
or mistake `/share/post` message-slice creation for full-conversation sharing.
Keep these gaps in the [remaining batch](web-ai-private-native-remaining-batch.md).
