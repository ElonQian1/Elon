# ChatGPT private conversation project move

## Capability

- ID: `android_chatgpt_private_conversation_project_move_v1`
- Status: completed, production-default, device verified
- Provider: ChatGPT Web
- Adapter: `245`
- Verified release: `v1.1.1514 (1514)`
- Official fallback: existing context-bound project move menu

## Product behavior

The production conversation sidebar keeps its native cached project picker. Choosing a
destination no longer navigates to the conversation or opens and polls several official DOM
menus. It submits one project move transaction in the persistent identity WebView, keeps the
native conversation UI visible, and updates the cached directory only after the server or the
target project directory confirms membership.

The previous official DOM coordinator remains intact. It is entered only after an explicit
"official confirmation" choice, when the private transport is unavailable, or when an
ambiguous result needs repair. This preserves compatibility without making DOM latency the
normal interaction.

## Transaction contract

One explicit destination selection issues one same-origin `PATCH` for the selected
conversation with the canonical project identifier. Android receives only a correlated
command receipt; cookies, authorization headers, page runtime tokens, and private content
remain inside the identity WebView.

Only one mutation write may be active. A timeout or transport failure starts bounded read-only
reconciliation against conversation metadata and the selected project's directory. The app
never automatically replays the move. Repeated failures open the existing short circuit and
offer the official menu instead.

## Verification

The versioned JavaScript transaction tests cover the exact single-write body, canonical
project IDs, metadata confirmation, target-directory confirmation after a timed-out write,
invalid-input rejection, and correlated bridge completion. Kotlin tests cover the typed MCP
command, consumer port, capability baseline, production action routing, native picker handoff,
and the explicit old-coordinator fallback. The focused suite, JavaScript tests, and Release
build pass.

Release `v1.1.1514 (1514)`, adapter `245`, completed a reversible MCP-only device round trip
with exactly one forward write and one restore write. Read-only reconciliation confirmed that
the original project membership was restored, with no unknown recovery state, private-content
output, Cookie clear, or app-data clear. The accepted source commit is `982154792`; the
published APK SHA-256 is
`d695476fc764417692ce35005fb21fb9961a9b94c4a0af514bebd3e4e30fd115`.

The acceptance run also closed a timing mismatch exposed by the first safe attempt. A private
mutation can spend about 30 seconds across page-local authentication, the single write, and
read-only reconciliation, while command receipts previously expired after 20 seconds. Mutation
receipts now retain a 35-second budget, the production coordinator polls for up to 40 seconds,
and an exact correlated late result can settle a locally timed-out receipt. The smoke runner
treats `timed_out` as terminal and never waits indefinitely or replays the write.

This capability is complete and must not be re-researched or reimplemented without current
regression evidence. The older DOM activation coordinator is retained only as an explicit
compatibility and repair fallback; the native cached picker is shared by both paths.
