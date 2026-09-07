# Private project-member conversation sharing

Capability: `android_chatgpt_private_project_conversation_share_v1`.
Status: implemented for authenticated personal-account conversations in an
already shared, non-sensitive project; offline verified; grouped Android
compilation, native consumer/MCP tests and production-phone acceptance pending.
This is not a completed capability or a live permission/access verification.
Adapter 284; shared transaction module 3; member resolver module 1.

## Official evidence

The inspected public asset `conversation-small-hiw4wce20lu6te81.js`
(SHA-256 `296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1`)
contains `Gkt`: the account's `normalizedAccountUserId` or the thread's
`sharedProjectConversationOwner.id` determines the owner; project metadata and
account type decide whether the member-specific modal is used. Health,
temporary, quorum and workspace contexts are not this personal member flow.

`https://chatgpt.com/cdn/assets/0ec7d136-iapesa5f61fnsuwq.js`
(SHA-256 `c720e9928fbca61b5e9c8a4b19eef020f1e68cc554d8598e281880552958d72c`)
implements `ShareProjectChatModal`. Its copy action calls shared export `nz`
with the server conversation ID, gizmo payload and owner ID. It describes
access by existing project members, including future messages. This action
does not create a public share or mutate project membership.

The already-loaded shared asset `4813494d-hrplraurzfyvxb10.js`
(SHA-256 `89c95d937bac1191e91d5ceb4872eb0c328d39a98ce05399093a663f18921aa0`)
exports `HWe` as `nz`. It builds the `/g/{short_url_or_id}/shared/c/{server_id}`
URL with the `owner_user_id` query parameter. Export `Pz` contains the observed
recipient enum, including `Private: private`. Its `getGizmo` uses the existing
`GET /backend-api/gizmos/{gizmo_id_or_short_url}` contract.

The current-conversation header passes its current server ID into `Gkt`.
That explicit ID has priority over the inherited shared-conversation fallback.
The native route already identifies that current server conversation: the
resolver keeps it, uses the official shared owner when present, and binds the
inherited ID for drift detection. It does not blindly replace the current ID
with `continuingFromSharedProjectConversationId`.

These are public-source observations, not authenticated request captures.

## Production chain

- Reuse the existing production Share coordinator, consumer port, tracked
  `share_conversation` command and WebView identity context.
- Encode member-only consent in the canonical project conversation path.
  Conflicting sidebar project metadata/path is rejected. A plain public-share
  command cannot silently become a member command, or vice versa.
- Load only the inspected, already-loaded official runtime through the existing
  share contract. No new store, copied credential persistence or DOM click.
- Bind the document, identity, current route, project, selected branch, normalized
  user, shared owner and inherited conversation state. Recheck around the bounded
  metadata GET and before returning the official URL builder's output.
- Require a matching project and an observed non-private recipient, or the
  legacy nonempty subject list. Unknown/unread state is not proof of absence.
- Validate the complete URL, including project/slug, current conversation and
  owner. Cache one successful link for at most 60 seconds while its binding
  remains current. This cache does not grant access; server membership still
  controls whether a recipient can view the link.
- Confirm and display member-only sharing in native UI, including future-message
  visibility. Copy/system share distributes the already-resolved official link;
  it is not a substitute API and never adds members or creates public access.
- Preserve the distinct `project_share_link_ready` receipt. A dedicated parser
  accepts only validated share URLs up to 512 characters, avoiding the generic
  160-character truncation. Other actions retain their existing limit.

The accompanying URL fix parses official full page URLs before comparing their
conversation identity, in both native policy and MCP dispatch. Previously the
relative-path parser rejected a valid `https://chatgpt.com/c/...` current page.
External origins and mismatched project routes remain rejected.

## Verification

- 131 focused Node cases pass across public sharing, member sharing, deletion
  and metadata mutation. Member cases cover scope/owner selection, bounded GET,
  exact headers, no public/permission write, cache reuse/expiry, late-response
  drift, single-flight and the concatenated production asset bundle.
- The original pure Kotlin share-policy suite reproduced one current-URL
  failure before the fix. The native receipt baseline then reproduced two
  failures in nine cases, including the long member URL truncation.
- Nine focused pure Kotlin/JUnit tests now pass, compiling the actual path,
  share policy, receipt validator and protocol-detail implementation directly
  with cached Kotlin 2.0.21. This is not an Android/Gradle build.
- The native consumer/MCP tests include member consent and tracked dispatch,
  but these broader Android-dependent tests were not compiled or executed.
  An attempted standalone run of the existing full protocol test lacked its
  Android protocol dependencies; it is not included in the nine passing tests.
- ADB currently reports no device. No new APK was built/installed, no real member
  URL was distributed and no real account/project permission was changed.

## Remaining acceptance and scopes

In the grouped APK, use an authorized synthetic conversation in an existing
shared project. Open Share from the production sidebar, verify the member-only
confirmation, copy result, exact owner/current conversation, and native return.
Verify current-member access and non-member rejection without changing project
membership. Confirm no `share/create`, public publication or permission mutation.
Check the ordinary public path once for regression, then preserve both results.

Guest, workspace, private/non-shared project public sharing, temporary-chat
sharing, link management/revocation and unsupported sensitive project types
remain distinct code gaps. Do not report them as implemented or widen this
member-only consent to cover them.
