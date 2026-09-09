# ChatGPT operation readiness batch, 2026-09-09

Base: `7216dff5505a495effa8472c499449575eda0795`.
Capability: `android_chatgpt_operation_readiness_v1`.

## Implementation

- Shared native-consumer/MCP entry now classifies local cache, current-document,
  account read and existing composer-transaction requirements explicitly.
- Conversation/navigation cache reads no longer require bridge READY. Stale
  page-local navigation option handles are omitted, not presented as usable.
- File library, conversation file lists and download resolution can enter the
  existing private identity owner when the document is current, even before the
  UI reports a logged-in account or renders a composer. Credential validation,
  account binding, bounded acquisition and exact handle checks stay in the reader.
- Model/tool discovery, existing conversation opening, stop and cancellation
  no longer depend on global composer READY.
- Direct background model requests and project-membership probes use the same
  existing document/directory access predicates.
- Existing write and microphone guards remain. No provider protocol, cookie,
  login storage, system alternative or independent proxy code changed.
- Native capability evidence treats unknown identity and stale adapter generation
  as recovery states; an existing cached option remains available during recovery.

This does not claim every private operation is independent of official runtime
state. Remaining boundaries are explicit in
[the operation policy](../chatgpt-operation-readiness.md).

## Verification And Delivery

- Targeted Release JVM tests: 93 passed across 13 suites, no failures or errors.
  Logged command: `webchat-operation-readiness-final-20260909` (289.7 seconds).
- Existing private auth-context and library-catalog Node regressions both passed.
  These cover the identity owner reused by the new admission policy; they are
  deterministic tests, not a claim of successful live provider requests.
- Release: `1.1.1603` / code `1603`, published from
  `e6a52ecd56e88ed9a4e0402afe54f587f0a3cad9` after the grouped Release build.
  Logged command: `publish-webchat-operation-readiness-20260909` (433.4 seconds).
- Online `version.json` independently matched the version, source SHA and local
  APK SHA-256 `8aea0d102eb47c6559caf977f33b92636174742f0484de0c1c0b1a04f81c5503`
  (40,031,749 bytes). Core commits: `d7c395195`, `f05d35d32`.
- Device: deferred; no microphone, upload or personal conversation mutation is
  required for this policy batch's offline regression tests.
- Automatic device installation was disabled for this invocation only; no
  installation or measured device-latency improvement is claimed. The publisher
  reported optional LAN firewall and worktree cleanup warnings; task cleanup is
  handled separately by the required finish workflow.
