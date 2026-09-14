# Writing Save Acceptance Guard

Date: 2026-09-14. Source baseline: `61c349c4f`.

## Scope

The production native Writing Block/code editor, local copy/edit/history/export,
and accepted ordinary-conversation cloud save remain unchanged. Their completed
scopes are recorded in [the capability page](../chatgpt-writing-blocks-native.md).
This batch changes acceptance tooling, not Android runtime behavior.

The old `smoke-chatgpt-writing-block-save-ui.ps1` cleanup could close the native
editor and restore another conversation after an unconfirmed cloud write.
That could obscure the outstanding operation and make a subsequent acceptance
run unsafe. A native status label alone also did not independently establish
that the observed receipt belonged to this attempt.

## Changes

- Require one new successful `writing_block` receipt with Boolean `ok=true`
  and `writing_saved`, together with the original page generation, conversation,
  completed source message and Writing Block position.
- Reject old/ambiguous receipts, concurrent sends or mutations, unknown streaming
  or dictation state, authentication/route changes, and write/readback/sync uncertainty.
- Gate both successful acceptance and cleanup on that evidence. An uncertain
  attempt stays in its editor and sets `recovery_required`; no save replay,
  additional write or automatic navigation is introduced.
- Keep screen-awake lease restoration independent of the cleanup decision.

## Evidence

`writing-save-guard-final-20260914-181450-202`: 67 checks passed in 1.9 seconds,
including strict-mode missing evidence and cleanup wiring. Fixtures are synthetic.
The project-route positive fixture tests the evidence policy only; it is not a
project cloud-save device pass.

Phone MCP read-only checks observed adapter 395, authenticated production native
ChatGPT, no draft, no streaming, and idle voice/dictation. This batch sent no
messages, performed no cloud saves and did not change project membership.

No new APK build or device write acceptance was run because no APK source changed.
Project, typed-widget, library-linked and temporary save acceptance remains pending;
the existing personal Writing fixture does not prove those variants. The known
ordinary editor/export/save results must not be repeated merely to fill this gap.
