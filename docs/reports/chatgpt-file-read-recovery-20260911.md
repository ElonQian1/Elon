# Conversation file-list recovery

Scope: explicit native conversation attachment reads and their refresh sheet.
This is a correction to existing functionality, not a new private protocol.

## Evidence And Limits

Two initial 1657 reads failed before the separately accepted citation download.
Their generic receipts do not establish the cause. The read-only baseline
`file-index-baseline-1657-20260911-124229-177` subsequently returned HTTP 200 in
2,406 ms and a two-file index. Conversation and draft were unchanged; no writes
occurred. Its top-level command status was mistakenly read from the receipt
envelope and is null, not a successful command assertion. The HTTP/index
observations remain valid. Do not relabel the earlier failures as proven timeouts.

## Verified Code Defects

- Explicit `account_read` network errors used a 10-second-or-longer cooldown;
  timeouts used 60 seconds. The UI offered retry without a useful failure reason.
- Footer actions always dismissed their sheet. Refresh then created another
  sheet, cancelling observation of the existing read and allowing redundant
  command dispatch. The underlying GET already had single-flight protection.
- File-index receipts flattened non-identity failures to `files_read_failed`.

## Correction

- Policy v3 keeps the existing account/background separation and persistence.
  Only explicit account-read network/timeout failures use a two-second manual
  retry cooldown. Background budgets, authentication/context rejection,
  server rate-limit and parse protections are unchanged. Existing longer
  persisted protection is never shortened or cleared during migration.
- Transport v27 emits bounded `files_read_timeout`, `files_read_network`,
  `files_read_rate_limit`, `files_read_http` and `files_read_parse` categories.
  Unknown error bodies are not rendered. There is no automatic retry or write
  replay and no additional transport implementation.
- The production attachment sheet refreshes in place, keeps cached rows during
  loading/failure, and ignores additional refresh clicks while its read is
  pending. Owner/epoch guards remain. Shared footer actions still dismiss by
  default; only this refresh action explicitly stays open.
- Adapter 340 delivers the updated modules. Audio, dictation, proxy, Cookie and
  login behavior are untouched.

## Verification

- `private-file-read-recovery-node-20260911-125605-022`: four Node files,
  17 runner tests passed. The file projection suite additionally reports
  19 internal cases. Tests cover transient explicit retry, no scheduled retry,
  unchanged identity/rate-limit protection, safe receipts and late-result ownership.
- An existing request-lifetime test had not loaded the split directory modules.
  Its fixture now uses the current document-token, project owner and terminal
  cursor contract. It asserts one actual shared read instead of identical public
  wrapper promises. Production directory code is unchanged.
- `file-read-recovery-jvm-20260911-125832-978`: Release compilation and five
  `WebChatConversationFilesPresentationTest` cases passed, zero failures/errors.
  These include cached failure rows, safe labels, refresh ownership and the
  default-dismiss contract for other footer actions.
- Java semantic-menu acceptance helper compiled. The existing inventory script
  supports `-NativeMenu -VerifyRefreshInPlace`: wait for the initial read, tap
  Refresh three times, compare the Android window identity, preserve cached
  rows, and inspect successful native command receipts. If all three taps were
  sampled while loading, exactly one new read command is required.
  Deterministic injected failures are not a controlled live network outage.

## Normal 1658 Delivery

- Published and installed through wireless ADB on the trusted Xiaomi, without
  clearing data, at normal 1.1.1658 / code 1658 / adapter 340.
- Source: `23658f56b6d6cb8948dc3c7f7ade6cde8411df48`.
- APK SHA-256: `ed7979319a91d636f9563d07a503b8b3b8066f7142e6e3d3809a11b63ab53b51`.
- `file-read-recovery-release-340-20260911-130738-409`: Release build, vital
  lint, publication and unattended update passed. The auxiliary post-publish
  worktree cleanup reported a missing `Branch` property; publication and device
  version verification still completed. Task finalization is recorded separately.
- `file-refresh-native-1658-20260911-131543-931` stopped at `device_locked`
  before opening any native menu. Zero sends/downloads and no awake lease or
  conversation change occurred. UI acceptance is **deferred**, not passed.
  Resume the installed package after unlock; do not rebuild for this boundary.

Reuse the completed citation download scopes. This correction does not complete
mounted/cloud references, broaden model/account coverage or address thermal work.
