# Fresh Retry Admission

Capability: `android_chatgpt_fresh_regeneration_v1`.
This diagnostic does not complete or enable the independent writer.

## Evidence Gap

Build 1723 rejected independent native regeneration with `scope_unsupported`
before dispatch. The existing official runtime completed its retry. There was
no independent variant POST, so repeating a write cannot explain that rejection.

The current change adds `private_protocol_probe / regeneration_admission`.
It captures the same composer, last assistant turn and model trigger used by the
production native regenerate command, then runs the existing admission contract
without constructing a request or entering a writer ledger. It does not change
drafts, invoke retry callbacks, arm a trial, prepare proof or start a stream.
Concurrent inspections share one read and return within five seconds.

Both fresh context modules retain their original predicates. Scope rejections
now carry a bounded stage: route, owner, composer readiness, privacy, preparation,
project, branch/config, model, user identity/content/parent/channel/recipient or
user/reply metadata. Native receipts accept only the closed schema, stage and
code sets; no content, IDs, headers, credentials or arbitrary errors are exposed.

## Ownership

Base: `9a84407879d9a4d2b52645a28fea89444fa61a80`.
Existing dirty worktrees contain old DOM-send and new-conversation edits in the
adapter. They were reviewed and left untouched. This change adds only a separate
diagnostic command branch there. The stale 117-to-118 adapter version edit is
already superseded by the current 391 baseline; this candidate uses 392.

## Verification

- `fresh-retry-admission-full-contract-20260914-144951-717`: 289 Node tests
  passed, covering context, sends, attachments, temporary/project scope, retry,
  streams, stop/recovery and command wiring.
- `fresh-retry-admission-upgrade-20260914-145403-647`: six command-wiring
  tests passed after verifying upgrade from the prior orchestrator version 11.
- `fresh-retry-admission-android-20260914-144836-447`: Release Kotlin/Java and
  unit-test compilation passed; all 12 `ChatGptWebPrivateProtocolEvidenceTest`
  cases passed with zero failures, errors or skips (348.9 seconds).
- Source-size, document-modularity and whitespace guards passed.
- Wireless ADB initially identified the expected Xiaomi, then MCP bootstrap
  and a separately bounded call-state read timed out. No new message, retry,
  installation or disruptive recovery was attempted. Live admission is deferred.

This is a code-only diagnostic batch for the next grouped APK; no separate
diagnostic-only APK was published. Installed/published build 1723 is unchanged.
The original failed regeneration acceptance remains failed until an independent
owned stream and history reconciliation are actually observed.

## Subsequent Device Evidence

The preceding code-only status describes the first batch. The diagnostic was
subsequently published in `1.1.1724 / 1724`, source
`12847d8ee4e19ea6b39c06a4803117886169388d`, adapter 392. APK SHA-256:
`daac8819ca47419b9232902233b3d7b099d0b9892db6b186d86ccd89cb5cdae1`
(40,470,611 bytes). `fresh-retry-admission-release-20260914-153206-024`
completed Release compilation, packaging and publishing in 554.8 seconds.

Early installation guards timed out before installation. A bounded MCP service
bootstrap restored access; fresh call, voice, draft and attachment checks passed.
`fresh-retry-admission-install-service-20260914-155008-014` then installed with
`-r` and verified build 1724. Its final summary failed because the reopened home
surface had no adapter yet, not because installation failed. Native semantic
actions reopened the existing ChatGPT chat, without clearing data or reloading
an existing website session. Normal Release CDP is disabled by design; no
research/debug flag was enabled to bypass that boundary.

`fresh-retry-admission-live-native-20260914-155217-369` completed in two seconds:
`scope_unsupported / model`, adapter 392. Page generation/URL, native draft and
message identities/content were unchanged. It issued no send or regeneration.
This locates the admission failure, not a server or network failure.

## Nullable Model Resolution

The pinned September 12 public source gives a concrete missing case. Export
`azt / zwn` returns null outside its optional work-model path. Official composer
`A1t` selects `resolved ?? requested ?? conversationModel`, and applies the model
default effort only when that optional resolution is non-null. The independent
retry incorrectly required the resolver result to equal the requested model.
Its test fixture always echoed the requested model and missed the ordinary null
case. The live stage identifies model admission, but does not expose which value
was returned; the missing null branch is established by source and regression.

Context v4 now retains the committed retry model for a nullish resolution and
does not invent a work-model default effort. Model permission, changed non-null
resolution, explicit override, account/branch/owner, stream and write-once guards
remain unchanged. Adapter 393 loads this correction. No writing editor, voice,
proxy or production transport-default code was changed.

- `fresh-retry-model-red-20260914-155915-137`: three new cases reproduced failure.
- `fresh-retry-model-verified-20260914-155944-784`: 62 tests passed, including
  pinned source hashes and the official nullish fallback/effort expression.
- `fresh-retry-model-transactions-20260914-160041-219`: 128 tests passed,
  including ownership, stop/recovery, history and accepted runtime retry.
- Zero failures or skips in both passing runs. Corrected APK/device verification
  remains pending; this is not an independent regeneration completion marker.
