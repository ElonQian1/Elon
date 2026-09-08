# Runtime generation state

Capability: `android_chatgpt_runtime_generation_state_v1`.
Status: implemented, offline verified, shipped in 1554. Production send/settle
workflow passed; reader-path activation itself is not device-confirmed/completed.
This is a read-only official-runtime state reader, not a new message transport.

## Problem and scope

[APK 1552 acceptance](reports/chatgpt-runtime-release-1552.md) received the exact
test reply and a completed private stream, but native UI remained generating.
A stop command then reported no current generation. The precise live DOM
subcondition was not captured, so residual CSS is a candidate, not a proven
handset root cause. Existing policy treats any visible `main .result-streaming`
or empty last assistant placeholder as active despite a completed private stream.

The new reader reuses the committed composer resolver and versioned shared-module
bindings already used by runtime stopping. It neither invokes submit/stop nor
changes official stores, drafts, credentials or media tracks. Its result is
tri-state: active, confirmed completed, or unknown. Unknown is never idle.

## Confirmation contract

- Require a completed private stream with a concrete message ID.
- Resolve the current mounted conversation using the existing identity boundary,
  including positively recognized guest sessions.
- Require the official current leaf to equal that stream message, and reject
  conflicting known conversation IDs. Cached completion from another branch is
  not sufficient.
- Match the shared composer request ID to the official thread request ID.
- Require the exact recognized enum and getter contract; preserve active text
  requests and all realtime modes.
- Confirm no active request plus absent or `UNREAD` async state. Recheck document,
  URL, mounted node, controller, leaf and request ownership after observation.
- Do not release while any native private submit, regenerate, stop or relay
  transaction still owns an unsettled write.

The streaming policy uses this reader only for a conflict between private
completion and DOM-active/pending state. Ordinary streaming and settled DOM
do not invoke it. There is no new timer, dense poll, HTTP call or reload.
One startup cache warm reuses the existing bounded, single-flight module loader;
the read itself only uses already-resolved modules.

## Source evidence

The retained current public shared asset SHA-256 is
`48563cd22f0dafe6c0b89220348fa3add81ff3abb82a62ed9d68a04d569cc375`.
The existing versioned aliases resolve to public exports `mN`, `oN`, `Il`, `Lx`
and `R7`: thread lookup, thread request lookup, active-request membership,
conversation async status and status enum. Current `Il` (`sQ`) checks the
active-request store for a non-null request ID; `Lx` (`SH`) is a conversation
signal initialized to null. `R7` assigns STREAMING/UNREAD/realtime modes 3-7.
No minified export is inferred from its old name or written into by this reader.

## Verification boundary

The old streaming policy fails the new residual-marker/empty-placeholder case.
After correction, 129 focused Node runner cases pass across runtime state,
streaming settlement, existing send settlement/observer, stop wiring and runtime
text submission. New reader cases cover a matching completed leaf, absent
request ID, active text/voice, unknown modules/enums, changed conversation/branch,
mid-read ownership changes, pending writes and no repeated module import.

Release compilation, package/source comparison and pinned Xiaomi replacement
passed. Two consecutive production sends received the exact reply and settled
in 6.473s and 4.308s. The specific reader branch has no live receipt, so these
samples prove workflow behavior, not that this reader caused the improvement.
See [1554 acceptance](reports/chatgpt-runtime-release-1554.md).

Neither send accepted private text POST/runtime submission. The first retained
`submission_not_ready`; the second reached `conversation_route_mismatch`.
Those are separate pending runtime transaction issues. Google stays last, and
the broader Goal remains active.
