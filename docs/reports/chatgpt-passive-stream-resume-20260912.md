# Passive official stream continuation

Capability: `android_chatgpt_passive_stream_resume_v1`.
Code: implemented, enabled with the existing private stream observer.
Verification: offline verified and published/installed; live sending acceptance
failed before resume coverage could be established. Do not mark device verified.
This is passive observation of an official request, not an Android HTTP sender.

## Evidence and scope

Retained public runtime `web20260911_b` supplies the contract:

- Composer `8b34dbc2-fpy4mlfnxc115y6k.js`, `sXt`: the official client posts
  `{conversation_id, offset}` to `/f/conversation/resume` with its own credentials.
- Composer `nH`: retry remains owned by the official stream pipeline, preserving
  the higher-level decoder. Our observer never calls it or retries a write.
- Conversation `conversation-small-ft205i7yqa6zc2nj.js`, `Eun`: acknowledgement
  increments the resume offset for each data event, excluding response metadata.
  Encoding and resume-token events count even when hidden from the UI decoder.
- Conversation `UM`: JSON-serializes the body, ignores ping/empty events, and
  treats a stream ending without `[DONE]` as interrupted rather than successful.

The existing observer captured only the initial conversation POST. It called
`session.finish()` on EOF or reader rejection, incorrectly completing partial
text while the official client could continue it through a different response.
The new regression suite reproduced both failures before the change.

## Implementation

- Fetch tap 3 captures only the bounded, exact resume cursor, never ordinary
  send bodies or request headers. It returns the original response unchanged.
- Transport 19 accepts a resume only when the document token, current route,
  original request sequence, conversation, generation and exact offset match.
- Complete delta documents survive interruption. Incomplete SSE bytes are
  discarded before continuation, and a fresh TextDecoder owns each response.
- Only `[DONE]` or existing explicit official completion evidence completes
  an answer. EOF and failed readers retain received text, report interruption,
  and leave official snapshot reconciliation/watchdog behavior intact.
- Invalid encoding, mismatched conversation, stale/new-turn ownership or
  unknown resume metadata cannot append to the current answer. These cases
  remain under official recovery; no credential/token copying or request replay.
- Only the observer's cloned reader is cancelled. The official reader remains
  untouched. No timer, background polling or additional HTTP request is added.
- A previously captured tap 2 is preserved in that document rather than
  replacing the fetch reference the website already holds. Resume support begins
  with tap 3 on the next normal document load; ordinary observation still works.
- Page adapter 360 loads the revised assets through existing assembly.

## Offline verification

Twelve focused Node test files pass (258 tests), including 26 continuation cases
and 15 reader-interruption/ownership cases. Coverage includes repeated resumes,
mid-event disconnect, heartbeat offsets, late same-conversation requests,
malformed cursor/body/encoding, terminal deduplication, external origins,
auth failures and real ReadableStream clone cancellation isolation. Existing
delta visibility, submit transaction, stop and watchdog checks also pass.

The older interruption fixture was missing its extracted delta-document
dependency. It now loads the same dependency order as production and asserts
that transport interruption is not synonymous with answer completion.

Live network interruption/resumption has not yet been observed for this change.
Synthetic tests do not establish server resume timing or independent POST
viability. Do not declare the broader private-sending goal completed from this
observer improvement, or disable VPN to manufacture acceptance evidence.

## Normal APK and device checkpoint

- APK `1.1.1679` / code `1679`, adapter `360`, source
  `66122dcf3ee6a65b12c025a1ea29d25a3fd14ac1`.
- SHA-256 `1daaf5851a7334ca689de343d7d5b80b78426ffe2556566f958b0ebe01ccbe80`.
- Release log `chatgpt-stream-resume-release-20260912-064205-776` passed in
  458.4 seconds; Gradle completed in 6m39s. Normal `com.elon.app` updated in place
  over wireless ADB; package version independently checked. No data/login reset.
- Production `social_ai` bound with adapter 360, authenticated and composer
  ready, native/official draft lengths zero and streaming false before testing.
- First native send probe `chatgpt-stream-resume-native-1679-20260912-065032-414`
  failed its 90-second reply condition; the runner completed restoration.
  Its original timeout did not retain enough send diagnostics to classify it.
- One distinct diagnostic probe, not a replay of the first send,
  `chatgpt-stream-send-diagnostic-1679-20260912-065836-776`, also failed. Final
  diagnostics: ready/composer-ready, streaming false, 3 native messages,
  user-marker matched, assistant-marker not matched, assistant text length 4.
  The send receipt succeeded through DOM with `template_unavailable` and
  `runtime_not_observed`. This is not evidence that a private resume succeeded.
- The native smoke now includes bounded non-content failure diagnostics before
  restoration clears the send receipt. Its contract check passes.
- Read-only `runtime_assets` returned a truncated 96-file manifest containing
  `c2675c8c-o59yc0xo7p9m3q3o.js`, but not the recognized composer/conversation/shared
  role names (which the collector prioritizes). `document_state` was complete,
  with one visible editable prompt, one form and no incomplete collection.
  This supports a runtime-discovery investigation, not a proven new export map
  or a conclusion that network connectivity is universally healthy.
- Fetching that public asset from the PC failed to connect within 25 seconds.
  No guessed profile was enabled. Reuse the native protocol probe and public
  asset analysis on the next batch; do not repeat the completed Canvas case.

The original conversation and awake setting were restored by both runners;
subsequent canonical state was ready, streaming false, dictation false and
empty input. Remaining priority is current official runtime discovery and
normal native send/reply acceptance, then a genuine resume sample if available.
