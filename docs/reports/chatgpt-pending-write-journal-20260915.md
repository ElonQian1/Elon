---
capability_id: android_chatgpt_pending_write_journal_v1
implementation_status: integrated_candidate
verification_status: targeted_js_android_verified_device_pending
delivery_status: normal_1765_published_installed_candidate_disabled
---

# Pending Write Recovery

Delivery update: grouped normal Release 1765 / adapter 428 compiled and was
replacement-installed, with native readiness confirmed. The generated normal
BuildConfig still has `CHATGPT_FRESH_TEXT_JOURNAL_ENABLED=false`; installation
does not accept active-write process recovery or its banner. See the
[grouped release receipt](chatgpt-runtime-bindings-20260915.md).

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
  plain-text recovery uses one history request. Attachment recovery now has the
  additional digest check described below.
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

### Attachment Proof Follow-Up

Journal/store 2, request 8 and adapter 423 close a reproduced source defect:
after recreation, the earlier journal constructed a history owner without the
original attachment lease. Exact user identity and a terminal answer could
therefore settle an attachment send whose file references were absent or changed.
The accepted live sender already checks its immutable file lease; that path is
unchanged. Two new negative tests failed against journal 1 before the correction
(`fresh-journal-attachment-red-20260915-141233-771`).

- Request preparation exposes a read-only signature of its actual selected
  document IDs, image pointers and materialized mounted references. Only its
  SHA-256 hash is persisted, never file names, prompt text or content bytes.
- Record schema 2 requires an explicit digest or null for a plain-text send.
  Reference ordering and unrelated provider-added fields do not change the
  signature; duplicate, missing, extra or replaced references fail verification.
- Attachment recovery hashes a read-only history response, then requires the
  same exact references in the guarded application response. It shares the
  existing 12-second total deadline, route/owner checks and no-replay barrier.
  Plain-text recovery still needs one read and rejects unexpected file references.
- The storage namespace is unchanged so schema 1 records cannot become invisible.
  Old sends cannot be assumed to have no attachments and remain unresolved.
  Old regeneration records retain their existing full-user-digest/reply-ID proof.
- The request/transaction integration test compares the persisted digest with
  the actual POST body, recreates the journal against shared storage and proves
  terminal history with one original POST and no replay.

This remains the same opt-in candidate, not a new accepted capability or a
default promotion. The change adds no upload, DOM selector or provider endpoint.
The final scoped **431-case JS regression passed**, zero failures, skips or
cancellations (`fresh-journal-attachment-final-20260915-142018-412`, 4.4 seconds).
It includes the preserved legacy regeneration proof. No Android compilation,
APK publication or active-write process-kill acceptance was done in this follow-up.

### Read-First Follow-Up

Journal 3, transaction 34 and adapter 425 correct an admission-order defect. The
old transaction captured a fully sendable parent/model/draft before inspecting
its journal. After recreation, a missing parent or unavailable editor could
therefore stop the very history read needed to restore that parent. The new
negative cases reproduced this against the old order
(`journal-read-first-red-20260915-151709-892`).

- A separate read-only context binds the committed conversation, identity,
  document, profile, project and privacy scope. It loads only the reviewed
  conversation runtime, never the editor or selected model, and does not require
  a completed parent before reading history.
- Native send intent first calls `recoverSelected` using that context. This
  reuses the original journal's account filtering, exact terminal user/branch/
  attachment proof, timeout, cancellation and compare-before-remove behavior.
- An empty journal issues no history request. An unresolved record stops before
  normal send capture, preparation or POST; a resolved record permits one fresh
  capture using the now-restored parent. No prompt is replayed and no new journal
  record is created by the read-only step.
- The read owner expires on account, document, route, runtime profile, committed
  owner, privacy/project changes or a competing official write. Temporary and
  history-disabled chats still skip durable recovery. The public read operation
  can consume a context with no draft, model or `current()` send-admission method.
- Production asset registration precedes transaction creation. Both transaction
  module and instance are version 34, preserving the reinjection ledger guard.

The final focused run passed **333 JS cases**, including actual journal/context
integration, recreated sender order, an unavailable DOM draft reader, legacy
records, attachments, normal/new/project/temporary/tool sends, Stop and recovery.
Log: `journal-read-first-final-20260915-152359-278`. The first wider run caught a
module/instance version mismatch; it was corrected before the final passing run.
The candidate flag remains unchanged. No Android build, installation, POST replay
or active-write process-kill device acceptance was performed for this follow-up.

### Page-Entry Follow-Up

Adapter 426 / fresh transaction 35 wires the existing journal into production
snapshots, independently of Send and of `composerReady` / `privateSendReady`.
The separate recovery-session module reuses the exact account/conversation
history proof. It emits only an aggregate state through the canonical snapshot
and the existing native composer status banner, never a fake assistant message.

- An empty journal causes no history GET and no checking banner. A real pending
  record changes the banner to checking; verified history removes the record.
- Unknown results stay unconfirmed. Storage failures are unavailable, not an
  empty ledger. The banner offers inspection, never an automatic Send retry.
- Cold committed-owner mounting gets at most three attempts. Network/history
  failures wait for an online/foreground event, with a 15-second cooldown that
  survives network flapping. There is no idle history polling.
- New writes cancel passive recovery before claiming ownership. Account, route,
  document/token changes, suspension and disposal invalidate late callbacks.
  Native voice relay setup/takeover defers history recovery without changing
  media. Current send admission, Stop and voice/dictation routes are preserved.
- The committed conversation is still required as an identity owner; this does
  not infer a new server ID, fake first-send readiness or bypass unknown runtime
  mappings. The feature remains an opt-in candidate until process-death proof.

Verification: 507 focused JavaScript tests passed, including the full fresh-text,
regeneration, private-input and pending-recovery wiring suites. Log:
`pending-recovery-batch-final-20260915-160035-776`. Release Kotlin/Java compilation
and 36 Android tests passed (protocol, asset manifest, production recovery policy
and pending-write presentation), with no failures/errors/skips. Log:
`pending-recovery-android-final-20260915-155750-818`, 279.7 seconds.
The first JavaScript run found two fixture setup errors (new-root identity and
fake-timer settling), both corrected. The first Android test compile found an
incorrect test event property; main Release compilation had already passed.
No APK publication, installation or microphone action is part of this batch.
Separately released 1.1.1763 remains adapter 424 and excludes this candidate;
that release is not evidence of installation or acceptance of adapter 426.

### Device Boundary

The earlier read-only MCP check confirmed normal 1.1.1762 / adapter 421 installed,
authenticated and ready in the production native chat, with empty draft and
idle voice/dictation/streaming. It does not contain this opt-in source follow-up.
The existing idle-body recovery smoke then stopped before navigation/force-stop:
the handset was locked. A separate display-state read confirmed awake=true and
keyguard_showing=true. Zero messages were sent; original/awake restoration passed.
Receipt: `conversation-process-recovery-1762-20260915-140845-230`.

- No genuine active-send process-kill recovery has been accepted on a device.
  Existing idle process-recovery smoke is insufficient for that claim.
- Reconciliation now also runs from production page snapshots and online/resume
  events before any Send intent. Offline tests cover both readiness flags being
  unavailable; actual process-death recovery and native banner rendering on a
  device remain unaccepted. The normal Send safety gate was not loosened.
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
- Attachment completion now requires the request's reference digest as well as
  exact user identity; this is not device acceptance of Library/mounted or
  expanded attachment combinations.
- Normal APK 1765 now includes this candidate with its flag disabled. Grouped
  build/install/readiness evidence is recorded above, not active-write durability.
  The included subtitle fix separately passed one supervised search-response
  sample on 1765; this is not evidence of journal durability.
