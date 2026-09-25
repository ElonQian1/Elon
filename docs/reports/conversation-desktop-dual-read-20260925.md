---
version_status: current
reviewed_at: 2026-09-25
implementation_status: in_progress
---

# Claude Desktop and two-device conversation reader audit

This supersedes completion assumptions, not the existing private history/media
implementation. Only structural evidence is retained. No conversation body,
title, account identifier, device serial, attachment name, credential or signed
URL is recorded here.

## Missing desktop integration repaired

The user clarified that the consumer is Claude Desktop. The previous runtime
integration targeted Claude Code, which does not configure the desktop client.
The installed desktop configuration had four other MCP servers and no personal
conversation reader.

`scripts/web-conversations/register-claude.mjs` now performs a guarded merge of
only this reader, preserves other tools/preferences, backs up the original,
uses immutable assets outside task worktrees, and retains explicit per-device
selection. Grants remain a list of explicitly authorized conversations. The
actual local configuration was updated and the installed desktop was restarted.
Loading the server and an actual Claude model tool invocation remain separate
acceptance steps; neither is inferred from writing configuration.

A verifier also launched the exact command/arguments/environment from the
desktop configuration, without inheriting the shell environment: initialization,
all four tools, one-conversation scope and Win connection succeeded. This is a
configured-command test, not a Claude model invocation. The official desktop
deep link was requested to prefill a read-only test prompt; it does not send it.

## Repeatable acceptance

The existing Windows verifier now accepts an explicit Win or APK source and
rejects pages or bytes from the other source. The new
`scripts/web-conversations/acceptance.mjs` entry point reads all pages and each
attachment, verifies bytes against returned SHA-256, compares two completed
devices, and writes only structural receipts outside the repository. A failed
device does not prevent testing the other. Matching partial reads remain partial.

## Live observations

| Case | Current evidence | Result |
|---|---|---|
| Codex native MCP discovery | All four scope/connect/read/asset tools available; one authorized conversation | passed |
| Win access | Installed `0.3.69+b3945a7486e0e269b46ede1b51d5f1a477e952aa`, native host and frontend available | passed |
| Win complete pagination and image bytes | 23 pages, 45 blocks, 25,443 text characters; 8 image references, 4 distinct image hashes | passed |
| Win source-format coverage | 90 candidate messages, `unsupported_message_content` still reported | partial |
| APK installed identity | Registered Xiaomi identity verified over wireless ADB; final full run on package `com.elon.app`, version `1.1.1815 / 1815`, verified before and after | passed |
| APK initial access | Ready adapter, authenticated official conversation, 90 candidate messages and 8 attachment references | passed |
| APK full pagination and image bytes | After the device became unlocked, 23 pages, 45 blocks, 25,443 characters and all 8 image references completed | passed |
| Win/APK comparison | Same authorized-conversation binding, text digest, character count, four unique image byte/digest records and remaining gaps | passed |
| APK source-format coverage | Same `unsupported_message_content` as Win | partial |
| Claude Desktop registration | Installed `1.1.4173`; original four servers preserved; reader registered with a stable asset path | passed |
| Claude Desktop actual load and model invocation | Restart observed, but no reader log or model call; native Computer Use launch returned no targetable window, and a fresh window inventory remained empty | user_action_required |
| Ordinary files/PDFs | Existing synthetic production-module tests only; target conversation supplied images | offline_verified |

The first Win attempt read five image references then encountered a transient
timeout. A fresh full registered-stdio run completed all images and pagination.
Its plaintext SHA-256 was
`6559110de18bbe27528926c55618421a557902caead2a682ff080ed7469f70fb`,
matching earlier evidence. All four image hashes and byte counts matched the
[rich reader report](personal-web-conversation-rich-reader-20260925.md).
The new local acceptance run ID is `422bb2ea-7637-4da4-9ceb-4aab9668b5be`.

The complete reader/registration/acceptance Node regression suite passed 53
tests. This includes preservation of existing desktop configuration, malformed
configuration and competing-writer rejection, explicit source pinning, digest
verification, and preserving partial/failed status across device comparisons.

The secure keyguard was an observed condition concurrent with the earlier APK
1814 stall, not proof that every pending request has that cause. The device later
became unlocked and had been updated to 1815 by another delivery. This task then
completed the explicit APK verifier in 94 seconds. Its receipt is
`96ed4a8c-4d05-4579-b8b3-b846d0d8d979`; the same binding, text and all image records
match the Win receipt above. Both receipts remain `partial` because of the source
format gap. This does not prove locked-screen operation or attribute recovery to
a particular code change. No downgrade, data clearing, credential transfer or
lockscreen bypass occurred.

## Remaining work

1. Confirm the Claude Desktop reader actually initializes and exposes all four
   tools, then perform an authorized model tool call. Desktop startup evidence
   alone is insufficient. Its reader server log and account-ready event have not
   been observed; inspect the actual client UI before assigning a cause.
2. Identify unsupported message shapes using structure-only evidence before
   extending projection. Do not discard unknown records to make coverage pass.
3. Test actual ordinary-file/PDF reads only with a separately authorized suitable
   sample. Returning bytes is not proof of model interpretation.

The overall requirement remains incomplete. This batch closes the missing
desktop configuration and proves matching dual-device text/image reads. Actual
Claude client use and complete source-format coverage remain unverified.
