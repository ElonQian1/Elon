---
version_status: current
reviewed_at: 2026-09-25
---

# Win unattended conversation acceptance evidence

The existing exact-release update guard, installed launcher, semantic controls
and personal conversation MCP were already available. Earlier live verification
used temporary orchestration scripts. This delivery adds the durable,
repeatable [acceptance entry](../win-conversation-unattended-acceptance.md).

## Delivered code

- `9906c01a1`: fixed semantic control and fresh stdio reader adapters.
- `df5f7a435`: persisted orchestration, resume binding, lock recovery and CLI entry.
- No Win binary, APK, frontend or cloud API changes are required for this runner.
  It consumes the existing installed semantic control and reader capabilities.

The final combined Node test run passed **46/46** cases, including production
reader regressions and 17 acceptance workflow cases. Tests cover node/desktop
startup, token loss, exact activation, update intent before mutation, lost-reply
reconciliation, no duplicate update on resume, live/dead lock ownership, binding
mismatch, action receipt identity, navigation, pagination and attachment digests.

## Real Win run

The runner used the installed runtime
`0.3.69+b3945a7486e0e269b46ede1b51d5f1a477e952aa`.
Its initial and final control capability readbacks matched this exact identity;
both Tauri and frontend were available.

`reload_page`, `navigate /ai` and `capture_state` each returned successful
semantic receipts. Navigation also returned the required `/ai` route. A new
stdio MCP process exposed all four conversation tools and read the authorized
conversation without manual clicks.

| Evidence | Actual result |
|---|---|
| Initial full run | 276.7 seconds; read to end |
| Explicit persisted resume | 273.9 seconds; fresh read to end; same text/image digests |
| Pages / blocks | 23 / 45 |
| Plaintext Unicode characters | 25,443 |
| Attachment references / successful bytes | 8 / 8 |
| Distinct attachments | 4 PNG images |
| Content status | `partial`, exit 2 |
| Remaining gap | `unsupported_message_content` |

Plaintext SHA-256:
`6559110de18bbe27528926c55618421a557902caead2a682ff080ed7469f70fb`.
The four image digests and byte counts matched the earlier
[rich reader acceptance](personal-web-conversation-rich-reader-20260925.md).
The reader's `message_count=90` represents candidate source messages; it does
not establish full parsing of all those records.

The logged command wrapper reports a nonzero result because the runner
deliberately returns exit 2 for incomplete content. This is not a connection,
navigation or attachment failure; the remaining format gap is preserved.

## Recovery verification and boundaries

The explicit resume run used the persisted receipt and the same project,
conversation authorization and exact release binding. It returned
`workflow_complete=true`, repeated the three successful semantic actions and
fresh MCP reads, and reproduced all page counts and text/image digests. The
remaining content gap still produced `partial`/exit 2, without repeated updating
or reuse of old cursors. The offline recovery failure matrix also passed.

The installed release already matched the requested target, so the live runner
skipped updating; this run does not claim another desktop restart. The prior
exact-release update and automatic reopen were separately exercised in the rich
reader delivery. Real update-while-guard-running recovery remains separately
unverified; timeout, lost reply and resume behavior are covered by deterministic
tests.

No account credentials, private messages, titles, conversation IDs, filenames,
signed URLs or attachment bytes are recorded here or in the runner's receipts.
Valid local login and working official-site networking remain prerequisites.
APK device acceptance and actual Claude model invocation are outside this Win
orchestration change and retain their previous pending status.
