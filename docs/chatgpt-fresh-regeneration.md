---
capability_id: android_chatgpt_fresh_regeneration_v1
implementation_status: implemented
verification_status: offline_verified
production_default: false
scope: existing_personal_plain_text_retry
---

# Fresh Regeneration

September 13 source candidate. The existing native regenerate button now has a
gated independent request path through the same fresh-text ledger, prepare/proof
provider, owned stream, stop and history recovery. It does not call the website's
retry callback and does not submit the previous prompt as a new user message.
The [accepted runtime retry](chatgpt-official-runtime-regeneration.md) remains
the production default until grouped native acceptance.

## Protocol Evidence

`test-chatgpt-text-dispatch-public-evidence.cjs` parses the three pinned
`web_20260912` public assets and checks their SHA-256 before examining definitions.
Downloaded provider JavaScript is not executed by the tests.

- Composer `TKn/nS` calls conversation `Iht/Wdr` for ordinary retry. It finds the
  original user with `getParentPromptNode`, retains the variant IDs, and requests
  completion type `Variant`. The parent is the existing user ID.
- Ordinary `oGn` supplies the selected retry model without feedback text.
  `Vdr/Udr` uses `comparison_implicit` for one existing variant, otherwise `none`;
  the ordinary path has no appended personalization/feedback message. Contextual
  feedback retry, map parameters, images, files and tool hints are excluded here.
- `FB` prepares action `next` with that user parent and no messages. `AB` then
  dispatches action `variant`, omitting messages and retaining variant purpose
  and message-follow-up support. The new request has its own conduit/proof/trace.
- `azt/zwn` is the reviewed requested-model resolver, exposed only for this
  profile by binding v26. Retry uses the previous answer's thinking effort or
  the resolved model's default; it does not borrow the composer current effort
  or service tier. The pinned `A1t` path and request-body projection are asserted.

Two initial evidence assertions incorrectly assumed a string quote and a ternary
form in the pinned minified source. They were corrected to the actual definitions;
only the passing evidence run below counts.

## Ownership And UI

`chatgpt_web_fresh_regenerate_context.js` reuses the committed native-command
owner and existing retry/menu eligibility contract. Initial admission still needs
the current composer and retry owner; this is not a composer-free implementation.
After admission, streaming ownership does not depend on rendered buttons.

The context records the account/document/conversation, original user and ancestor,
old variants, retry model and effort. Before writing it checks that the draft,
selection, prompt and model permission remain unchanged. Incidental timestamps
do not change prompt identity. The composer draft is never cleared by retry.

Only assistant IDs observed in this command's decoded HTTP/topic stream can prove
a new answer. Old variants are rejected before native projection. A new variant
created on another device cannot settle this command just because it appears in
history. Reconciliation requires the original user branch and a terminal owned
reply, then uses the existing official history/store hydration without reload.

Regenerate and send share one command ledger. Duplicate command IDs cannot write
twice, and a send cannot reuse a regenerate receipt. A pre-dispatch compatibility
gap may claim the accepted runtime once. An ambiguous dispatched request retains
its fence and uses read-only recovery, never another writer. Trial/state output
adds only the operation name, not prompt text, credentials or request headers.

The normal native stop route uses this preparation's conduit and retains the
observed partial answer. Stop before any owned assistant frame remains a separate
acceptance gap: an unchanged old answer alone cannot prove this retry completed.
Loss before any identifiable frame likewise remains explicitly unconfirmed.
No account data, Cookie, proxy core or accepted audio path is changed.

## Verification And Next Acceptance

- `fresh-regeneration-verified-20260913-143125-239`: 465 Node tests passed with
  zero failures/skips, including pinned source evidence, versioned bindings,
  new/existing/tool/project/temporary/attachment sends and accepted runtime retry.
- `fresh-regeneration-stop-20260913-143345-189`: 43 focused cases passed after
  adding real fresh-stop composition. This includes the new regression plus
  existing stop/recovery tests; it is not 43 additional production samples.
- `fresh-regeneration-final-20260913-143738-822`: 63 send/attachment/retry and
  stream-admission checks passed after adding operation-only observations.
- Final history review `fresh-regeneration-history-review-20260913-143922-316`:
  56 cases passed. Changed prompt content is rejected before hydration, while
  incidental timestamps and equivalent empty metadata do not block reconciliation.
- Integration composes the actual committed-owner contracts, context, request,
  transaction, stream transport/decoder/projection, stop and history reconciler.
  Only provider/network responses are synthetic. Cases cover one variant POST,
  original user retention, draft preservation, explicit trial, model/scope denial,
  lost response/tail, another-device variant, route changes and partial stop.
- Asset wiring and the assembled JavaScript parse pass. No APK build, device
  installation, live regeneration or performance claim is made by this batch.

Grouped APK acceptance: in a controlled ordinary conversation, retain a draft,
arm the existing `fresh_text_trial_start`, and tap the production native regenerate
button once. Verify one new assistant variant, no additional user turn, visible
streaming, retained draft and reconciled history. Repeat with native Stop only to
cover its distinct path, then restore the prior conversation. Use
`__elonChatGptFreshRegenerationEnabled=true` only for explicit testing; a trial is
one-command and does not enable the production default. Do not repeat the already
accepted runtime test and label it independent HTTP acceptance.

Projects, temporary/new contexts, attachments/tools, feedback retry, branch
selection UI and composer-free initial ownership remain outside this candidate.
Preserve their current paths; extend only from actual protocol evidence.
