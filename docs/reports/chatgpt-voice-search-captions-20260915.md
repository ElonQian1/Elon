---
capability_id: android_chatgpt_realtime_voice_data_channel_transcript_v1
implementation_status: regression_fix_implemented
verification_status: targeted_android_tests_passed_device_pending
delivery_status: included_in_installed_1765_voice_acceptance_pending
---

# Voice Captions After Search

## Observed Boundary

On installed normal Release 1.1.1759, the user reported continued spoken analysis
after a search preamble, without corresponding native captions. Read-only MCP
matched the current native conversation's final user question to the reported
topic. Its final assistant preview contained nine characters, with no subsequent
analysis message. The native surface contained 15 messages; the backing snapshot
still contained three. No transcript text was exported or retained as a fixture.

The native peer reported connected, remote audio present, data channel open,
106 received messages and 91 decoded transcript events. These counters alone are
not proof of the failing event shape: before this fix, rejected/unrecognized
frames did not publish updated counters. Audio completeness is user-reported;
the agent did not record or listen to the call. The call was not restarted or
ended for diagnosis. Reopening the native chat did not restart the peer.

## Confirmed Code Defects

The current official voice handler reconstructs `ChatMessageDelta` using the
shared JSON delta decoder. The reviewed Sep 15 public asset
`conversation-small-newrvr7nrx5tnmp4.js` has SHA-256
`039efa3e391652942a9ae5de7cc057eb1bc05c3afad32d851e14ac47b554470d`.
Its `chn` decoder, exposed to the voice handler through `yhn`, indexes previous
values by the provider channel identifier without a 0..15 identifier restriction.
It inherits channel/path/operation metadata, but not the previous payload value.

Our decoder incorrectly treated its 16-entry memory budget as an upper bound on
provider channel identifiers. Any delta with channel 16 or higher was rejected,
even if it was a complete new assistant message. It also inherited the previous
payload when the value field was omitted, potentially repeating caption text.
These are independently reproducible compatibility defects, not proof that the
exact reported live frame used a high channel: that requires a new live sample.

## Scoped Fix

- Accept valid nonnegative integer identifiers independently of cache size.
- Keep at most 16 recently updated channel states; accept full root replacement
  to reseed an evicted channel, but never fabricate a partial message from a
  patch whose base was evicted.
- Reject malformed/fractional/overflow channel identifiers without redirecting
  them to another channel. Missing payload values cannot replay previous text.
- Preserve the existing native decoder-to-conversation-bubble path and audio peer.
- Serialize transcript decoding/reset. Publish received-frame counts even when
  a frame does not decode into a caption.
- MCP adds numeric highest-channel, cached-channel and rejection counts plus an
  allowlisted rejection reason. No payload, text, credentials or audio is logged.

## Verification

Targeted tests cover more than 16 sequential channels, sparse high identifiers,
LRU bounds/reseeding, invalid identifiers, omitted values, and a synthetic
search-preamble -> tool -> assistant-delta -> assistant-final sequence reaching
native conversation bubbles without DOM/history refresh. Existing parser,
continuity and MCP tests are included in the same run.

Passed 34 Android Release unit tests, with zero failures/errors/skips:
`voice-search-caption-regression-20260915-120054-578`, 450.1 seconds.
Suites: delta decoder 8, parser 8, native bubble continuity 14, search pipeline 1,
MCP state 3. Source-size and whitespace checks also passed.

Live acceptance must distinguish received frames, decode rejections and native
message updates during the search response. Do not claim the user's exact live
regression resolved from unit tests alone. The user subsequently confirmed that
the call ended and installation is permitted.

## Release

Normal signed Release **1.1.1761**, source `656d0fd3e2410229cb9175702fa86d14d25e606a`,
is published at the normal APK endpoint. Receipt
`voice-search-caption-release-20260915-121154-403` passed in 563.5 seconds;
Android compilation, release checks, upload and remote package verification passed.
APK size: 40,562,991 bytes. SHA-256:
`93b16e1e83fb09986a129ac2860f83014932d6a22c4e25e563e6288d86c8ef12`.
The page adapter remains 421 because this is a native subtitle-decoder change.

The initial autodeploy was deferred while the phone was absent. On Sep 15 the
same trusted Xiaomi returned over wireless ADB. The saved package hash matched
the published SHA above; `adb install -r` succeeded and package-manager readback
confirmed version name 1.1.1761 and code 1761. No application data or login state
was cleared. MCP opened the production social AI chat, reporting adapter 421
with the bridge still restoring (`web_only`, adapter not yet current).

Subsequent MCP health and a four-second basic ADB shell probe timed out. This is
loss of the debug transport, not evidence of provider network failure or a
failed subtitle decoder. No new call/microphone session was started. Actual
search-to-spoken-answer caption continuity remains pending. A later mainline APK
containing this commit can be used; do not downgrade or rebuild 1761 merely to
repeat installation.

Later the separately published group-chat release 1.1.1762, source
`b643afa250a580a3faff717c25e1e7d6d7488928`, included this decoder fix and was
installed by the canonical release script. This task independently read back
MCP version 1762 and opened the production social AI surface: adapter 421 current,
bridge ready, authenticated, no login requirement and native voice idle. Debug
transport had recovered. No microphone was started and caption continuity is
still not device-accepted; the subsequent safe restart smoke found the screen
locked before any process stop.

After the user again confirmed the call ended, grouped normal Release 1765 /
adapter 428 was replacement-installed. Before installation, MCP confirmed idle
voice; afterward production native chat became ready/authenticated with 10 native
and 10 backing messages, zero voice frames/transcripts/rejections and no active
audio. See the [grouped release receipt](chatgpt-runtime-bindings-20260915.md).
The user was asked to start the blue realtime control and ask a search-requiring
question. No new spoken sample or post-search caption continuity has yet been
observed, so the exact live regression remains pending rather than passed.
