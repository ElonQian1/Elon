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

The candidate is now included in [grouped APK 1701](reports/chatgpt-private-grouped-1701.md),
built, published and installed. The initial phone check was locked; live retry
remains pending for the reasons below.
Use the existing `scripts/smoke-chatgpt-web-regenerate.ps1` with `-FreshHttp -NativeRetry`
and the pinned device/hardware serial. This combines native button/receipt evidence
with the versioned one-command trial, exactly one attempt, owned stream events and
reconciled history. It retains a controlled unsent draft and restores only after
the trial has no pending write and fixture cleanup is confirmed. Changed user drafts
and unknown results are left intact. The default runtime registry is not overwritten
by this candidate test. Offline cleanup and evidence contracts passed; this does not
promote the candidate or prove native Stop.

## Trial Wire Compatibility

The unlocked 1701 attempt `fresh-retry-native-1701-20260913-160727-147`
stopped during the controlled seed send, before tapping native retry. Its send
receipt was `private_text_v1:unknown:reconciliation_pending`; later diagnostics
returned `invalid_protocol_evidence`. Neither result proves a successful retry
or a rejected server write. No seed replay or retry was performed.

The diagnostic failure had a concrete cross-layer cause: JavaScript emitted
`operation` in its version 6 receipt, while Kotlin accepted only the earlier
version 6 field set. The transaction now emits version 7, which requires the
reviewed operation enum. Kotlin also accepts both shipped version 6 shapes and
version 5, without accepting unknown fields, types, operations or versions.
Pending older writers remain readable and are not replaced during reinjection.

A shared synthetic fixture now checks actual JavaScript transaction output and
the actual Android command-receipt parser against the same seven states. The
`fresh-trial-wire-node-20260913-164317-160` run passed 51 tests with zero
failures/skips; `fresh-trial-wire-android-20260913-164350-233` passed all eight
focused Release JVM tests with zero failures/errors/skips. The smoke contract
also passed 18 negative, three native-identity, two draft and nine cleanup cases.

Fresh acceptance checks that diagnostic state is readable and idle before any
fixture navigation or seed write. If a seed is not confirmed, the script keeps
that conversation for read-only reconciliation instead of restoring away or
resending. That ordering is covered by the smoke wiring test, not claimed as a
new device recovery test. This fix changes diagnostic compatibility and test
safety, not request construction or the production regeneration default.

Grouped release `1.1.1704 / 1704`, source
`face23e9b7d01ba3c4d9376e89f8318740af69dd`, was built, published and installed
without clearing data. APK SHA-256:
`53560e809616f5ccc9b6b8fe9e7e5578dd7a01b8e772cbea5af691720d453048`.
The authoritative `fresh-trial-wire-release-20260913-165723-679` run completed
successfully in 517.8 seconds; installed package metadata independently confirmed
1704. The publisher's separate worktree-cleanup warning does not imply failed
installation; task cleanup still uses the required finish contract.

On Xiaomi, the first post-install production-surface readiness wait exceeded its
40-second limit. A subsequent read-only check found native `social_ai/chat`,
provider `chatgpt_web`, ready bridge/composer, retained authentication, adapter
376 and no draft or stream. Without another reopen/reload, the actual native
`fresh_text_trial_state` command returned valid version 7, idle, no pending writer
and no armed trial. This confirms the diagnostic wire fix on the device, not a
cold-start performance pass. Zero new messages, regenerate commands or trial
arming were performed. Independent regeneration and its Stop case remain pending;
the accepted runtime remains the default.

Projects, temporary/new contexts, attachments/tools, feedback retry, branch
selection UI and composer-free initial ownership remain outside this candidate.
Preserve their current paths; extend only from actual protocol evidence.
