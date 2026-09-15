---
implementation_status: implemented
verification_status: targeted_android_tests_passed_device_pending
delivery_status: pending_grouped_release
---

# Native Send Confirmation Identity

## Reproduced Boundary

The native send coordinator compared the latest user text, message count and
latest user message ID without checking the conversation that supplied them.
A different conversation containing the same prompt could settle the active
command. Expanding private history could also confirm the previous user turn
again when the same prompt was sent twice. Missing route or message identity
could be masked by the increased message count.

These are deterministic production-code defects, not evidence that they caused
the user's voice-caption regression. That separate fix remains
[published in 1761, installation pending](chatgpt-voice-search-captions-20260915.md).

Baseline `send-snapshot-identity-baseline-20260915-124041-977` compiled the Android
production and test sources and ran eight cases: four unsafe confirmations
failed, four compatibility cases passed. The failure was assertion-based, not a
compiler or network failure.

## Change

- For an existing ChatGPT conversation, require the same canonical conversation
  ID before consuming a submission/completion snapshot.
- When the pre-send user message has a known ID, require a different nonempty
  user ID. More messages alone cannot prove a new user turn.
- Reuse `ChatGptWebConversationPath`; personal/project URLs of the same
  conversation remain equivalent. Query parameters do not change its identity.
- Preserve new-chat route adoption and the existing Google policy. No new
  transport, DOM wait, network request, timer or automatic POST replay is added.
- Ignore insufficient evidence while retaining the pending command and its
  existing read-only reconciliation path. A later matching snapshot can settle it.

## Verification And Delivery

The focused regression passed **45 Android Release unit tests**, zero failures,
errors or skips: identity 8, coordinator 14, ledger 10, send owner 13. Receipt
`send-snapshot-identity-targeted-20260915-125410-148` passed in 24.2 seconds,
reusing the production/test Kotlin compilation from the preceding run. Ignored
snapshots cannot permit a second dispatch; a matching completion removes the
pending watchdog. Source-size and whitespace checks passed. The existing large
remaining-batch index has a document-size warning; this report carries the detail.

The wider run `send-snapshot-identity-regression-20260915-124712-194` ran 47
cases, with 46 passing and one existing global ownership-contract failure:
`GroupWebAiExecutor.kt:79` directly calls the page adapter's `sendPrompt`.
Both that module and `WebChatSendOwnershipContractTest` are unchanged from base
`e96f8b009dacf4f766c705ccf54b70743ced125d`; the group module was introduced by
`ac79125fb`. The other ownership test (personal-chat shared owner) passed.
This batch does not weaken the guard or claim the whole suite passes. Group-chat
ownership requires a separate scoped correction; it is not private personal-chat
device acceptance and is not changed here.

This is source for the next grouped Android release, not a replacement for the
already published 1761 subtitle acceptance package. Device acceptance is pending;
ADB currently reports no connected device. No phone data or login state changed.

## Process-Recreation Gap

The audit does not claim durable in-flight recovery. `WebChatSendCommandLedger`
is memory-only; `ChatGptWebSessionStateStore` persists a safe route, not a pending
write. The fresh transaction's provider user-message ID and dispatch boundary
also remain owned by its page-local transaction. Restoring history after an idle
restart is not proof that an interrupted send can be reconciled after process death.

A durable implementation still needs an account-bound, versioned pending-write
record at the actual dispatch boundary, exact provider message/parent/route
identity, safe clearing rules and post-restart history reconciliation. Persisting
only prompt text or a UI request number would be insufficient. Recovery must not
resend an uncertain POST, store runtime credentials, or persist temporary chats.
No speculative journal or incomplete persistence hook was added in this batch.
