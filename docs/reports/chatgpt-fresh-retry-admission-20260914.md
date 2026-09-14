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
