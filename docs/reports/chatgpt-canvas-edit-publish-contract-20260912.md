# Canvas Save And Publish Contract

## Delivered Scope

`android_chatgpt_private_canvas_shared_publish_v1` updates an **existing personal,
already-public Canvas link** to the latest original content. It reuses the
production shared-link manager and the [native source viewer](chatgpt-canvas-content-native-20260912.md).
It does not create a new link, edit the original, widen access, or silently copy
a shared snapshot into a different document.

- `transport=page_private_http`; existing WebView identity and header allowlist.
- `code_status=implemented`; content module v2, shared-links v6, share owner v7.
- `verification_status=offline_verified`; production UI/device publication deferred.
- `completed=false`; source batch only. Installed normal 1673 has neither this
  update action nor the source viewer; no live publication was attempted here.
- APK packaging and adapter release stamp remain part of the grouped acceptance
  build. The current source adapter stamp is 358, not a claim of a new APK.

## Actual Website Owner

The public asset chain is now resolved, not guessed from endpoint names:

1. `8b34dbc2-fpy4mlfnxc115y6k.js` lazily imports
   `08b6430b-dmctbpjfystgwrh0.js` as `CanvasFocusedViewManager`.
2. That manager imports `0b1acb52-kh29yq13e6iy6vk1.js` as
   `CanvasFocusedViewContent`, which delegates to
   `ac476d6c-n5q4k6410vjb4971.js`.
3. The latter contains the actual editor persistence, original rename, sharing,
   snapshot update and history owners. SHA-256:
   `2ac259a5c38a21c79a43fa49b1b6dab3c94a4081290f362d3cf199558f6b4af8`.

Assets were fetched from the observed `https://chatgpt.com/cdn/assets/` paths
without credentials and inspected as text, never imported/executed. Offsets below
are zero-based string offsets in this exact generation, not runtime selectors.

| Operation | Evidence and request contract | Source implementation here |
|---|---|---|
| Original rename | `zi` at 14366; POST `/textdoc/{textdoc_id}/rename`, body `title` | Not implemented |
| Original share lookup/create | Near 24304; GET/POST `/textdoc/{textdoc_id}/share`, unwrap `shared_textdoc` | Not implemented |
| Refresh existing published snapshot | At 24565; POST `/textdoc/shared/{shared_textdoc_id}/update_to_latest`, no body, unwrap `shared_textdoc` | Implemented, pending device acceptance |
| Change audience | At 24815; POST `/textdoc/shared/{shared_textdoc_id}/update_access`, body `shared_textdoc_access` | Not implemented; never used implicitly |
| Save original body | `Da` at 32465; POST `/textdoc/{textdoc_id}`, body `version`, `content`, serialized `comments`; result `version` | Not implemented |
| Original history | `za` at 35728; GET `/textdoc/{textdoc_id}/history?before_version=...`, unwrap `previous_doc_states` | Not implemented |

The existing request client supplies `/backend-api`. Shared IDs and original
textdoc IDs are different owners; neither can substitute for the other.
Publishing is a snapshot operation, not live mirroring of future source edits.

## Why Editing Needs Its Own Owner

The real editor queues saves (`Aa`, 32737), carries the last successful version
forward, and updates an optimistic local document store before persisting. The
three-second debounced owner (`Na`, 33729) flushes on visibility/pagehide/unmount
and waits for in-flight saves before dependent actions.

Comments are not arbitrary fields to forward unchanged. The imported `lvt`
resolves to `ver` at 2198840 in `conversation-small-ft205i7yqa6zc2nj.js` (hash in
the source-viewer report). It serializes comment positions against current
content; `uvt` resolves to `zH` at 2198676 for the inverse conversion. Original
saving must preserve comments, resolve version conflicts, and reconcile local
pending edits. The existing source-viewer cache is not this edit store.

Next implementation should start with an original conversation textdoc owner
using the already-observed `/conversation/{conversation_id}/textdocs` read and
real persisted IDs/versions, then source editing and conflict-safe saving.
Do not rediscover the panel, use `infer_metadata` as a save endpoint, or write
an edited public snapshot back under its shared ID. Rich editing, comment
position mapping, original first-publication and revision restore remain gaps.

## Native Update Behavior

- Existing share window adds `web-chat-canvas-share-update`; confirmation uses
  `web-chat-canvas-share-update-confirm` and `web-chat-canvas-share-update-cancel`.
  Cancelling performs no write and restores the selected link.
- Canonical `share_conversation` / `update_account` requires a Boolean user
  confirmation, `resource=canvas`, a selected personal shared ID and live ticket.
  Composer/model/DOM readiness is not a prerequisite.
- Fresh GET checks public access, moderation, sensitive-content restriction and
  a real version. A cached prior success cannot authorize changed access.
- Selection is consumed before one POST. The existing single-flight owner and
  unknown-write cooldown prevent duplicate publication or concurrent revocation.
- A fresh GET must match the POST result's exact ID, version, title, type and
  body. Decreasing/unknown versions, changed account/document or timeout do not
  emit success or trigger a second transport.
- All three requests share an 18-second deadline under the 20-second native
  command budget. Each request is at most seven seconds; no background polling.
- Success emits the existing content event, not the body in a receipt. The
  native viewer checks request ID and selected shared ID. Returning refreshes
  the consumed list ticket; dismissal/epoch change rejects a late UI reopen.
- Failure offers read-only verification and the existing official entry, never
  automatic replay. No new public link, source write or audience change occurs.

## Verification

- Five Node suites: 197 checks passed, including 31 new publish cases; stale
  selections, fresh access checks, deadlines, malformed responses, readback
  conflict, missing display channel, duplicate taps and no replay covered.
- Canvas/share/content/publish production semantic source guards passed.
- Android Release production and test compilation passed. Five selected JVM
  suites passed 24/24 tests (zero failure/error/skip), including the new native
  confirmation/dispatch tests. This is not an APK release or device acceptance.
- No personal Canvas modified, no Cookie/app data cleared, no proxy/voice change.

Grouped device acceptance must use a disposable owned public Canvas: cancel the
confirmation once, update once, compare original/latest and public snapshot,
verify the unchanged link and source, then return to the prior conversation.
Do not republish an unrelated existing personal document for acceptance.
