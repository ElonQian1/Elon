---
capability_id: android_chatgpt_fresh_temporary_text_dispatch_v1
implementation_status: implemented
verification_status: offline_verified
production_default: false
scope: authenticated_personal_temporary_plain_text
---

# Fresh Temporary-Chat Dispatch

September 13 extension of the [independent sender](chatgpt-fresh-text-dispatch.md)
and [new-conversation ownership](chatgpt-fresh-new-text-dispatch.md). It reuses
their page-local HTTP, request ledger, stream decoder, stop and history recovery.
It does not duplicate the accepted [temporary-chat selector](chatgpt-private-temporary-chat.md).
The existing ordinary-conversation production default remains unchanged.

## Reviewed Protocol

The hash-pinned `web_20260912` shared/conversation/composer bundles are the same
retained public sources recorded in `test-chatgpt-text-dispatch-public-evidence.cjs`.
They are parsed, not executed. The new assertions verify:

- Composer `AB` forwards `history_and_training_disabled` separately from
  `is_do_not_remember`. Neither flag may be invented from a native label.
- On a new history-disabled conversation, shared `wJ` gates the optional
  personalization field. Shared `OJ` reads the per-client selection through
  `f2`/untracked access. An enabled selection of `false` must remain explicitly
  false in `temporary_chat_requests_personalization`, not be dropped.
- Preparation and dispatch share these fields. Existing temporary follow-ups
  omit the new-chat personalization field, as does a disabled official gate.
- Composer `FB` also considers the separate local memory flag. The previously
  accepted temporary selector observes legacy local `is_do_not_remember=false`;
  it is not evidence that temporary mode is off.
- Composer `C1t` excludes history-disabled conversations from its normal first
  navigation/directory-update block. A returned server ID is still bound to the
  same registered client object. The temporary homepage must not become `/c/id`.

Binding v24 exposes these reviewed helpers only on this profile. Context v6,
request v5, reconcile v4 and transaction v13 use the existing modules. No new
asset injection or giant session-entry implementation was added.

## Admission And Ownership

Only the canonical `https://chatgpt.com/?temporary-chat=true` homepage is admitted,
with a committed current conversation, personal workspace, real temporary
selection and history-disabled runtime state. The same homepage supports both
an empty registered client/root and a continuing server-bound temporary chat.
Project, shared/business, guest, attachments and unreviewed continuation settings
are not silently admitted. Tool combinations are not part of this acceptance.

Initial capture still uses the committed composer host. Later checks use the
owned object, document/account, navigation entry and in-memory state; a detached
composer element does not itself invalidate the request. Changed identity,
privacy selection, current conversation or homepage entry prevents late writes.
New sends require both temporary and new-conversation admissions.

The first decoded owned stream binds its server ID once. Native text uses the
existing SSE/v1 pipeline. Stop consumes only an acquired owned conversation and
conduit; follow-up uses the authoritative current parent. Duplicate native
commands cannot create another POST. Later unsent drafts are not overwritten.

History recovery requires the correct conversation, user/parent branch, no
project owner, and `is_do_not_remember=true`; an explicitly false
`is_temporary_chat` is rejected. This is conservative response admission, not
proof of the server's current temporary-history availability. Missing or
incompatible history leaves recovery unresolved instead of replaying the send.
Successful temporary finalization stays on the same homepage and never calls
the ordinary first-navigation helper.

## Local Snapshot Privacy

The existing background session excluded the temporary homepage from its
per-conversation directory cache but could still save it as the provider's
last-page snapshot. `WebChatSnapshotPrivacyPolicy` now guards that store before
serialization/write and after decoding on restore.

Temporary, conflicting or unknown privacy query values and untrusted ChatGPT
URLs are rejected. Ordinary snapshots and Google's existing policy are retained.
Skipping a temporary save does not erase the previous ordinary snapshot. A
legacy temporary cache is no longer restored; this change does not claim to
securely erase previously written bytes. It does not clear Cookies or app data.
This guard covers this Android store, not all internal provider/browser storage.

## Verification And Delivery

- `fresh-temporary-owner-final-20260913-131655-391`: 279 Node tests passed,
  zero failures/skips. Includes pinned public contracts, binding compatibility,
  normal/project/new/tool regressions and temporary module composition.
- Temporary cases cover first and follow-up send, acquired ID, streaming,
  stop/partial reply, duplicate command, later draft, privacy-field combinations,
  account change during binding load, privacy/owner loss before and after ID
  adoption, history refusal and no ordinary navigation.
- `temporary-snapshot-kotlin-compile-20260913-131538-023`: focused Kotlin 2.0.21
  compile passed. `temporary-snapshot-kotlin-test-20260913-131546-047`: seven JUnit
  tests passed across the new privacy policy and existing bounded cache policy.
  This is not a full Android store, APK or device test.
- An initial public-source test used an overly narrow `if` pattern; the reviewed
  source contains a preceding log call in the same condition. The assertion was
  corrected without changing runtime code. Only the final passing run counts.

Use the existing one-command `fresh_text_trial_start`, or explicit
`__elonChatGptFreshTextTemporaryEnabled === true` together with the new-chat gate
when applicable. No production default is promoted by offline tests.

Grouped acceptance remains pending: build one APK, use the production native
temporary selector, send one controlled turn, confirm complete native text and
an unchanged temporary route, follow up, stop, leave temporary mode, and verify
that its text is absent from native restored history. Confirm actual server
history privacy fields. Preserve the prior ordinary conversation and login.
No APK, account write, microphone operation or thermal measurement occurred in
this source batch. Temporary attachment/tool combinations remain separate.
