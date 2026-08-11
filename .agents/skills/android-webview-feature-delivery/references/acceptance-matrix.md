# WebView Feature Acceptance Matrix

Use this reference to plan code work and record device evidence without conflating the two.

## Status Vocabulary

| Field | Value | Meaning |
|---|---|---|
| `code_status` | `implemented` | The native path and tests exist. |
| `code_status` | `partial` | Some required behavior is absent or knowingly incomplete. |
| `code_status` | `official_fallback` | The official WebView intentionally owns this behavior. |
| `code_status` | `missing` | No acceptable implementation or fallback exists. |
| `verification_status` | `offline_verified` | Static, unit, contract, and build evidence passed. |
| `verification_status` | `device_verified` | The current APK passed the required real-device case. |
| `verification_status` | `user_action_required` | The next check needs supervised user interaction. |
| `verification_status` | `deferred` | Verification could not run because the required environment was absent. |
| `verification_status` | `failed` | Current evidence shows incorrect behavior. |

## Capability Row

Record one row per user-observable capability:

| Capability | Code status | Verification status | Code gap | Verification gap | Risk | Owner |
|---|---|---|---|---|---|---|
| Example: send message | implemented | device_verified | | | reversible | native chat |
| Example: account purchase | official_fallback | user_action_required | | supervised official flow not run | sensitive | official WebView |

Split broad labels such as "settings" or "tools" into independently testable controls. Keep stable IDs and expected state transitions with the row or its linked test case.

## Risk Classes

| Risk | Examples | Execution rule |
|---|---|---|
| Safe | manifest read, bridge version, navigation, pagination metadata, fallback open/back | Batch in one smoke run. |
| Reversible | isolated message, model choice, tool choice, disclosure toggle | Run only when authorized; restore prior state. |
| Sensitive | login, account choice, password, upload, microphone, delete, logout, purchase | One case at a time under user supervision. |

Never lower a risk class only to automate more cases.

## Acceptance Sequence

1. Audit the capability matrix and resolve genuine code gaps.
2. Pass focused tests, repository guards, and Android build.
3. Commit and publish one APK tied to one Git SHA.
4. Install or update without clearing cookies or app data.
5. Run the safe batch and save structural receipts.
6. Run authorized reversible cases and restore state.
7. Run supervised sensitive cases individually.
8. Update code and verification statuses separately.
9. Report passes, failures, and deferred cases in one consolidated result.

## Structural Evidence Record

Use a record shaped like this and omit private values:

```json
{
  "capability": "send_message",
  "code_status": "implemented",
  "verification_status": "device_verified",
  "risk": "reversible",
  "apk_version": "1.2.3 (123)",
  "git_sha": "0123456789abcdef",
  "control_id": "chatgpt.send",
  "expected": "one isolated message is submitted and streaming begins",
  "observed": "receipt=accepted, streaming=true, message_count_delta=1",
  "recovery": "returned to the previous conversation list"
}
```

Do not include message text, account names, cookies, tokens, file contents, or screenshots containing private conversations.

## Stop Conditions

Stop the batch and preserve evidence when any of these occur:

- The installed APK version or Git SHA does not match the acceptance target.
- The device identity changes unexpectedly.
- Login state disappears or the official origin changes outside the allowlist.
- A command lacks a stable semantic target and would require coordinate tapping.
- The action becomes irreversible or sensitive without explicit supervision.
- Recovery cannot restore the prior safe state.
- The provider page changes so the adapter cannot produce the declared protocol version.

Mark the affected case `failed` or `deferred`, keep the official WebView fallback available, and return to code diagnosis only when the evidence identifies a real implementation gap.
