# Project mixed-media acceptance

## Current Status

- Capability: `android_chatgpt_private_new_project_attachment_upload_v1`.
- Scope: new project conversation, fixed text/PNG/PDF bundle.
- Implementation: existing private uploader, project permission and origin
  binding, native pending-file/send owners are reused unchanged.
- Verification: scope/receipt/restore harness contracts passed; real project
  attachment send is **deferred**, not completed. A subsequent device window
  reached the project URL but stopped before upload at composer readiness.
- Delivery: latest independent package read is normal `1.1.1601` (code `1601`),
  superseding the initial 1599/1600 observations. The harness-only checkpoint
  needed no APK. Subsequent recovery/readiness corrections are published together
  as `1.1.1602` below, not installed or device-accepted by this task.

The preceding goal turn made verified progress: ordinary new-chat mixed media
passed on normal 1598, with one accepted native send and all three content reads.
Do not repeat that scope; [retained evidence](chatgpt-media-batch-1596.md).

## Device Window

The authorized Xiaomi was online and unlocked. Initial foreground was another
application. One authorized return to the production native chat confirmed an
authenticated, ready, empty ordinary conversation and five cached projects.
The bounded refreshed native directory still exposed five projects and no
obviously dedicated acceptance project. No titles or conversation text were
emitted. No project was created and no existing conversation was modified.

Before the planned project navigation, foreground changed again. The guard
stopped the operation with `foreground_changed` **before project navigation,
upload or send**. Do not classify this as an attachment protocol failure or ask
for another login. Do not repeatedly steal foreground from the other workflow.

### Subsequent Window: Project Readiness

One later authorized return confirmed the empty production ChatGPT composer:
authenticated/ready, zero messages/draft/pending files, not streaming. The native
`open_web_chat_project` action accepted an observed directory path. Readback
matched that exact project homepage without a query; no old conversation was
opened and no attachment was staged or sent.

The 30-second readiness window did not pass. Subsequent structural readback had
`authenticated=true`, `login_required=false`, `adapter_current=true`,
`page_generation=4`, `page_kind=feature`, `composer_ready=false` and
`bridge_state=connecting`. Native state then became `error` with the built-in
automatic-recovery-exhausted message. This is a page/composer-readiness failure,
not evidence of absent project capability, upload failure or expired login.

A single diagnostic presentation request was accepted, but the captured screen
already belonged to another application, so it provides **no official-page
diagnostic evidence**. Later resumed-activity readback confirmed that other app;
the attempted presentation restoration returned `main_activity_not_bound`.
Do not claim restored native presentation. No further foreground navigation was
attempted. The task-only ADB debug forward and handset capture were removed;
the normal APK exposes no WebView debug socket. No private request or credential
was exported. Readiness cause remains unproven: source inspection shows the
recovery timeout requires a ready composer, but does not establish whether the
observed page was blocked, unavailable, stalled or affected by backgrounding.

At the next uncontended device window, restore native presentation first, inspect
the same project readiness boundary using existing MCP state, and obtain actual
official-page evidence only if needed. Do not repeat upload or build steps:
neither ran in this window. Do not enable debug access in the normal APK merely
to diagnose this one case. The user has been asked to pause other phone testing.

## Resumed Investigation And Recovery Classification

The September 9 resumed window reconnected the pinned Xiaomi through wireless
ADB. Native presentation was observed with authentication retained, current
adapter, the same project homepage and no ready composer. Foreground subsequently
changed to other applications again; no attachment was staged or sent and no
microphone was opened. A later package read returned `1.1.1600` (code `1600`),
superseding the earlier installed-version observation, not the earlier acceptance
evidence. Do not attribute this source change to that installed APK.

The app's read-only network probe returned a validated Wi-Fi/VPN network, a
successful ChatGPT TCP connection in 118 ms and homepage HTTP 200 in 1096 ms.
This is a connectivity sample, not an authenticated project-page probe. It
neither diagnoses the actual project page nor proves that its composer works.

Source inspection confirmed a separate reporting defect: every exhausted
recovery path emitted the same network/reconnection message. The recovery
coordinator now distinguishes navigation timeout, loaded-document/bridge
readiness timeout, an explicitly observed page error, a retry that could not
start, and unknown failure. ChatGPT renders that reason; a received page error
survives a subsequent error-document finish. A new navigation, successful
recovery or manual retry clears prior failure provenance. Existing retry limits,
delays, pause cancellation and send ownership remain unchanged. No readiness is
fabricated and no project capability is inferred from a missing composer.

Only two recovery callback lines change in `ChatGptBackgroundSession`; unrelated
uncommitted background-lease and new-conversation ownership changes in other
worktrees are not included. Google presentation and its private-protocol work
remain untouched. Release main/test compilation and all 14 focused JVM cases
passed (seven existing recovery cases plus seven failure-provenance cases).
Logged run: `webchat-recovery-causes-tests-20260909-140844-269`, 309.2 seconds,
no timeout or stall. Per the requested grouped delivery workflow, this fix is
source-verified and queued for the next combined APK, not yet installed. The
actual project-page readiness cause and project-media acceptance remain open.

## Actual Official-Page Evidence And Document Readiness

A later bounded diagnostic switched to `ChatGptWebOfficialActivity`, verified
that exact activity, and captured the selected project. The actual page showed
only a white background and a Retry button, with no composer. This is evidence
from the app's separate official fallback WebView, not acceptance of the resident
identity WebView or the native production UI. A bounded Chromium log read did not
expose a diagnostic error. The reason for this official Retry page remains open;
do not classify it as expired login, missing projects or an upload failure.

The ad-hoc diagnostic initially used the strict MainActivity foreground guard
after intentionally opening the official activity. That rejection was expected
for native acceptance, not evidence of a foreign app taking foreground. Keep the
strict native guard unchanged. After checking the exact official activity, one
Back action and explicit native-view selection restored the production native
chat, same project and empty draft. Readback confirmed native presentation,
foreground=true, composer_ready=false and bridge_state=connecting. No send,
upload, microphone, Cookie/data clearing or proxy change was performed.

Source inspection found a separate recoverability defect: directory refresh
required session READY, and navigation required a ready composer bridge even
when the current document adapter was already valid. The read request could be
accepted merely as a delayed retry, without actually dispatching a directory
request. A broken project composer could therefore also block cached-conversation
navigation and refreshing the existing private directory.

The existing access policy now distinguishes document operations from sending:
current-generation, same-origin, non-login documents allow navigation; private
directory reads also require an authenticated snapshot, or the existing ready
guest-composer path. Missing/stale adapters, foreign origins and explicit login
evidence are still rejected. Existing-conversation/project navigation reuses the
same path validation, cached preview, command owner and single-flight guard.
Deferred existing-conversation opens can dispatch on a current document snapshot
without falsely marking the composer/session READY. New-chat confirmation,
message/attachment submission, voice and permissions remain unchanged.

Targeted tests cover the failed-composer read/navigation boundary, stale adapter,
login/origin restrictions, guest behavior, cached preview, once-only deferred
navigation, missing command owners, invalid/concurrent paths, and preservation
of new-conversation gates. These source changes do not prove that the selected
project's actual Retry page is fixed or that project attachment upload works.

Release main/test compilation and all 37 focused JVM cases passed: 16 directory
refresh coordinator cases, 11 navigation cases and 10 access-policy cases, with
zero failures/errors. Logged run:
`webchat-document-navigation-tests-20260909-20260909-145059-771`, 301.5 seconds.
Source-size and document-modularity guards passed. This correction and the prior
recovery failure classification are intended for one grouped acceptance APK;
device acceptance is not yet claimed. The task-only handset capture was removed.

## Grouped Release 1602

The recovery failure classification and document-operation readiness correction
are published together as normal `1.1.1602` (code `1602`), source
`d525d8926835ab50984ea913b1982a2a7af3e22f`; code correction commit `2df5b628a`.
The publishing script passed in 461.1 seconds. Independent online metadata
readback matched the source and local APK SHA-256:
`4b6b6b55dd19b90ca1962723553d9662acbe84250696615a08f82a0536854736`.
Gradle version edits were restored; no version-number commit was created.

During packaging the phone foreground was a separate UI-tuner package, not
`com.elon.app`. Production MCP state did not provide a usable chat snapshot. The
task did not navigate or send against that test package. Automatic installation
was disabled only for this publishing invocation through a task-local config;
the user's global ADB settings were not changed. Final production package
readback is 1601. Replacement installation and the focused production UI checks
are deferred to an uncontended device window, not counted as passed. The APK
publisher's optional broad worktree cleanup warned about a missing `Branch`
property; task cleanup remains owned by the required finish command.

## Existing Harness Extension

`scripts/smoke-chatgpt-web-media-batch.ps1` now accepts `-Scope project_new`;
the default remains `ordinary_new` for explicit regression use. It requires the
already selected, empty project composer. It never selects an arbitrary project
or overwrites an existing chat/draft by itself.

- Require HTTPS `chatgpt.com`, default port, no userinfo/query/fragment, and an
  exact canonical project homepage path with optional observed slug.
- Reuse fixed native text/PNG/PDF staging, local removal/restaging, one send,
  actual content assertions and private upload/official-runtime send receipts.
- After the reply, request the existing read-only, fresh private conversation
  membership reconciliation against the captured project. Only a successful
  `probe_conversation_project` receipt observed after this request is accepted.
- Reject a changed conversation. The two official path forms for the same
  conversation may normalize without becoming a false context-change error;
  a different project prefix remains invalid.
- Restore the same project homepage, empty native/official draft and pending
  state, plus the previous awake setting. Stop on foreground change, active
  streaming/upload or a user-modified draft. No server file deletion occurs.

The contract tests cover ordinary/project scope parsing, foreign origins/ports,
userinfo, query/fragment, existing-conversation rejection, stale/failed membership
receipts, same-conversation aliases and foreign project paths. Existing checks
still reject answer-bearing prompts, multiple sends and unsafe cleanup actions.
These are offline harness checks, not real project-upload acceptance.

## Next Device Action

1. Install the published combined correction with replacement installation only;
   preserve Cookie and app data. Check production-native directory refresh and
   existing conversation/project navigation while the composer is unready; do
   not use the separate UI-tuner or official fallback activity as native proof.
   Then use production native controls to open an explicitly selected project
   homepage with no messages/draft/pending files. Keep original conversations
   untouched; use a separate new test conversation.
2. Run the existing logged command runner with this script and
   `-Scope project_new`, the current ADB transport and pinned hardware identity.
3. Accept only private upload + accepted send + all three content reads + fresh
   project-membership success + restored state. Inspect any failing layer once;
   do not rebuild or repeat previously completed ordinary/voice cases.
4. Record this narrow scope only after that actual pass. Synthetic remote files
   or the new conversation may remain. Project-file collection placement,
   read-only membership, existing project branches, ingest-image flags and other
   formats/sizes need their own current evidence; this harness does not prove
   those merely from a successful reply or conversation membership.

Google remains after the remaining ChatGPT acceptance gate. The full Goal is not
complete. Cookie, login, proxy core, existing voice/subtitles and application data
were not changed in this batch.
