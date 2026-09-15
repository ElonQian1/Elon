---
capability_id: android_chatgpt_pending_write_journal_v1
implementation_status: integrated_candidate
verification_status: targeted_js_android_verified_device_pending
delivery_status: source_only_grouped_release_pending
---

# Pending Write Recovery

## Scope

This addresses the memory-only pending-write gap identified in the
[send identity audit](chatgpt-send-confirmation-identity-20260915.md). It does not
replace accepted ordinary sends, idle history restoration, voice or subtitle
transports. It is not a claim that every process-recovery case is completed.

The production fresh sender now has an opt-in durable journal at its actual POST
boundary. `ELON_CHATGPT_FRESH_TEXT_JOURNAL=true` enables the candidate in a grouped
acceptance build. The normal build defaults to false until real process-recreation
acceptance; accepted independent sends retain their existing defaults. Keep the
candidate enabled across the restart being tested. Disabling it is not proof
that an outstanding write has failed, and must not be used to retry that write.

## Implementation

- Separate versioned storage and reconciliation modules. Reuse the existing
  reviewed private history loader, parent/branch proof and production send owner.
- Synchronously commit the record before the provider can POST. A fresh process
  reads it before preparing another write in the same conversation. It never
  resends the old POST, prompt or audio.
- Persist account hash, attempt/user/parent/conversation/project IDs, observed
  regeneration reply IDs and Stop acknowledgement, not prompt text, Cookies,
  headers, security tokens, attachments or audio. Temporary/history-disabled
  chats skip storage entirely.
- Account-scoped keys, exact schema, compare-before-update/remove, readback of
  synchronous writes, 16 unresolved records per account, 64 observed reply IDs.
  Unknown writes are not expired, evicted or replaced by a different attempt.
- Only exact terminal provider history may remove an old record. Wrong account,
  conversation, project, user, parent, branch and still-streaming data do not
  settle it. The first server conversation ID is adopted into new-send records.
- When recording is enabled, missing cold identity or capture failure cannot
  offer the legacy sender before checking the journal. An uncertain old write
  cannot be bypassed through an early pre-admission fallback.
- Regeneration also hashes the original question signature and requires a reply
  ID actually observed for that attempt. Its recovery does a read-only signature
  check before a second, separately guarded official history application. Normal
  send recovery uses one history request.
- Bound history reads to 12 seconds, abort on timeout/context change, and prevent
  late callbacks from hydrating a different owner. Recapture the current parent
  and generate a fresh request after recovery, before the next prepare/POST.
- Never treat HTTP acceptance alone as terminal completion. Keep the record on
  uncertain stream failure; remove it on verified reconciliation/Stop completion.
- Native pre-dispatch failures preserve the new unsent draft. Distinguish an old
  unresolved write from local recording failure, rather than marking the new
  draft sent or falling through to a second sender.
- Version 9 diagnostic receipts carry only a journal state code and recovered
  count. Native validation and shared JS/Android/PowerShell receipt fixtures are
  updated together while retaining older receipt versions.

## Verification

Synthetic tests exercise separate module instances sharing durable storage,
actual fresh transaction dispatch hooks, failure before/after POST, exact history
proof, recapture, account isolation, project scope, temporary storage exclusion,
regeneration identity, cancellation/timeouts/late callbacks and storage failures.
The final broad scoped fresh-contract run passed **414 JS cases**, zero failures,
skips or cancellations (`fresh-journal-syntax-20260915-135149-786`). The later
cold-admission correction passed the 58-case targeted transaction/journal/shared
wire set (`fresh-journal-admission-20260915-135503-012`). No private conversation
is used as a fixture.

The added diagnostic field initially exposed an actual JS/native wire mismatch:
the old native exact-field/version validator rejected it. The shared wire test
failed before versioning; version 9 and strict aggregate-field validation were
added on both ends without relaxing older receipt contracts. The final Android
Release build/test run, with the journal flag enabled, passed **19 tests**:
receipt policy 8 and trial/wire validation 11, no failures/errors/skips. Receipt
`fresh-journal-android-final-20260915-134743-863`, 438.1 seconds. PowerShell trial
and send-evidence contracts also passed, including rejection of unreviewed
version 10. Source-size and whitespace guards passed. This compiled the Release
production/test sources; it did not assemble or publish a new APK.

## Remaining Evidence And Limits

- No genuine active-send process-kill recovery has been accepted on a device.
  Existing idle process-recovery smoke is insufficient for that claim.
- Reconciliation is triggered by the next native send preparation; automatic
  startup projection of an unresolved-send badge is not implemented in this batch.
- If a new conversation POST may have succeeded but its server ID was never
  observed, the record remains unresolved on the empty home route. Navigating to
  the correct recovered conversation permits exact user-ID proof; this module
  does not scan all personal history or guess a conversation by prompt text.
- Regeneration with no observed new reply ID remains unresolved, not falsely
  matched to an old reply. Temporary chats intentionally have no durable journal.
- A missing/malformed/unreadable record cannot be silently treated as proof of
  failed delivery. Cross-process simultaneous writers are not claimed to be a
  distributed transaction lock. Automatic POST replay remains forbidden.
- The candidate uses the provider-origin WebView `localStorage` backing store.
  Synchronous set/readback is not a device-level fsync guarantee; abrupt death
  before Chromium flushes its storage is an additional acceptance boundary.
  Device testing must verify real process-kill retention before enabling the
  candidate, not extrapolate durability from the in-memory test storage.
- Attachment completion uses exact user-message identity here; this is not new
  acceptance of Library/mounted or expanded attachment combinations.
- Normal APK release and device verification will be grouped with the remaining
  batch. No additional APK was published for this candidate. Subtitle fix 1761
  was separately installed; its live voice acceptance is still pending.
