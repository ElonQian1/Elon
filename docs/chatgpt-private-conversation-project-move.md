# ChatGPT private conversation project move

## Capability

- ID: `android_chatgpt_private_conversation_project_move_v1`
- Status: completed, production-default, device acceptance pending
- Provider: ChatGPT Web
- Adapter: `245`
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

Device round-trip acceptance is recorded separately. Until a device is available, the
capability remains enabled from tested code but its verification string stays `device_pending`;
no device result is inferred from offline tests.
