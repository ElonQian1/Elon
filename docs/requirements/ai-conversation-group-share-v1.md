# Native AI Conversation Sharing V1

## Scope

Capability: `android_chatgpt_selected_conversation_group_share_v1`.

The source is the production personal Yilong AI ChatGPT chat, not a developer
page. Long press exposes the existing message actions and selection mode.
Selected messages become a frozen, group-scoped publication. This is not an
official provider share URL and does not grant access to the source session.

The delivered slice is selected messages, including selecting every currently
loaded message manually. It must not describe a partially loaded transcript as
the whole conversation. Existing official full-conversation sharing remains
separate. Google source sharing, public links, continuing another user's
conversation and automatic AI summaries are outside this slice.

## User Flow

1. Long press a message and choose multi-select, or forward one message.
2. Choose messages in the current chat. Selection follows stable message identity,
   not an index that changes when earlier history is inserted.
3. Merge-forward opens a preview with editable title and excerpt. Excerpts use
   selected content only. The read-only preview shows the selected message order.
4. Choose a group and explicitly confirm send. No message is sent by selection,
   preview, group selection or cancel.
5. The group displays one compact card with source, title, excerpt, count and an
   optional first selected image as cover.
6. Open the card in a full-screen native read-only conversation reader. Back or
   return-to-group closes only the reader, preserving the underlying group,
   scroll position, draft and attachment state. Image/detail back returns to the
   reader first. There is no live provider composer in the reader.

## Data And Rendering

Schema: `elon.ai_conversation_share.v1`.

The snapshot preserves selected markdown, complete writing/code blocks and
supported structured chart parts. Gaps between nonconsecutive selections are
marked as unshared content, not fabricated assistant replies. The right bubble
belongs to the sharing user, never the reader's account.

Images are copied from the selected message's existing app-owned cache into
authenticated group storage. No provider URL, cookie, request header, original
conversation ID, local path or runtime asset handle is serialized into the
snapshot. Cached preview images retain their available resolution; this slice
does not promise original-file recovery. Unsupported files, audio/video,
annotated attachments or unfinished blocks fail before publication rather than
silently becoming an incomplete shared conversation.

Both native and PWA readers treat content as inert rich text. They do not execute
HTML, automatically fetch embedded markdown images, or follow private resource
URIs. Ordinary code examples remain readable. Explicit public HTTP(S) link taps
use an allowlisted link handler.

## Modules

- `MainAiConversationShareFeature`: production entry and lifecycle orchestration.
- `ChatSelectionIdentity`: stable selected revisions and detached rows.
- `AiConversationShareDraftBuilder`, `Codec`, `Media`: selected-only snapshot and
  controlled image preparation; no provider controller dependency.
- `AiConversationSharePreview`, `TargetPicker`: confirm-before-write flow.
- `AiConversationShareApi`: first-party authenticated IO and persistent retry key.
- `AiConversationShareReader*`, `CardViews`: read-only native rendering.
- `store/articles/snapshots`: immutable group publication on existing social
  contents, revisions and media storage, not a duplicate content system.
- `ai_conversation_share.js`, `reader.js`, `rich.js`: inbound PWA parity.

## API And Safety

Base route: `/api/me/groups/:group_id/ai-snapshots`.

- POST creates the publication and its group message in one transaction.
- POST `/assets` uploads image bytes only, never a remote URL.
- GET `/:id` reads the snapshot; GET `/:id/assets/:asset_id` reads a referenced image.
- DELETE `/:id` withdraws the publication for its owner.
- Current group membership, distribution, sender and original unrecalled card
  are checked for every read. Other groups cannot reuse a snapshot or asset.
- Reserved card content cannot be forged or edited through generic chat sends.
- Retries reuse an account/server/group/document-scoped idempotency key.
- Read and image responses use private no-store and nosniff headers. Account
  changes invalidate in-flight presentation; denial clears displayed content.
- Only snapshot-created unreferenced image assets are eligible for opportunistic
  cleanup after 24 hours. Any publication reference or unknown ownership retains
  data. Ordinary article media ownership is not inferred.

Limits: 200 messages, 120000 characters per message/block, 500000 text characters
per snapshot, 256 parts, 12 images, 12 MiB per image and 48 MiB per upload batch.
Images must fit 4096 by 4096 pixels. Snapshot JSON is bounded to 2 MiB.

## Verification

Targeted Android tests cover frozen selections, source ordering and gaps,
selected-only excerpts, serialization, media bounds, malformed sources,
confirmation UI, native read-only layouts, identity and return navigation.

Rust tests under filter `snapshots::` cover schema, atomic create/idempotency,
group access, revocation, credential/resource isolation and orphan retention.

`scripts/test-ai-conversation-share-pwa.cjs` exercises the production renderer
at 320, 390 and 1280 pixels, rich content, protected images, search, browser back,
draft/scroll restoration, denial/retry/recall and inert untrusted content.

Physical Android acceptance and a live group-send are separate from simulated
layout, protocol and browser tests; never report them as completed without a
real device and an explicitly selected test group.

### Verification Record, 2026-09-15

- Android targeted suite: 74 passed, 2 skipped because this Windows test host
  cannot create the symlink fixtures. Main and test Kotlin compilation passed.
- Rust `snapshots::` suite: 29 passed. The broader `snapshots` filter also matches
  an unchanged compute-federation source-contract test whose transaction count
  is 7 while its assertion expects 6; that unrelated module was not modified.
- PWA production-reader browser checks passed at all three widths, together
  with the existing social browser test and 11 social recovery tests.
- Live group publication and physical Android visual acceptance remain separate;
  no real group message was sent by these automated tests.
