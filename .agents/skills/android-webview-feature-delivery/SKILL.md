---
name: android-webview-feature-delivery
description: Build or extend Android WebView-backed integrations that expose an evolving external web product through native UI, stable semantic manifests, MCP/ADB controls, official-page fallback, capability matrices, session recovery, and staged device acceptance. Use for ChatGPT, Gemini, vendor consoles, or similar WebView modules where code coverage and real-device verification must be tracked separately.
---

# Android WebView Feature Delivery

Deliver a WebView-backed Android feature as a maintainable native module without pretending that unverified behavior works. Preserve the official page as the authority and as the complete-function fallback.

## Invariants

- Treat the user's local web session and the official page as authoritative.
- Never export cookies, tokens, passwords, private request credentials, or private conversation content.
- Do not bypass login, anti-bot checks, account protections, or provider policy.
- Keep a full-screen official WebView route available for unsupported or newly changed functions.
- Never clear WebView cookies or application data during ordinary development or acceptance.
- Keep VPN and proxy-core changes outside the feature module unless that module owns the network stack.
- Expose user-visible actions through stable semantic IDs. Do not automate by screen coordinates.

## 1. Establish Ownership Before Editing

1. Read the active Goal, latest task messages, repository instructions, and current Git state.
2. Inspect active worktrees and compare exact files, symbols, and Git ancestry.
3. Classify overlap:
   - A capability already merged into the current base is reusable infrastructure, not duplicate work.
   - Uncommitted edits to the same symbols belong to the active owner until handed off.
   - Different modules that touch shared wiring require explicit ownership boundaries.
4. Send a concise handoff when another task owns overlapping files. Do not recreate its feature in parallel.
5. Use an isolated worktree and preserve unknown or unrelated files.

## 2. Plan a Modular Change

Run the repository preflight and read the returned edit root and finish contract. Before editing, publish a file plan with new files, modified files, estimated line counts, and whether any large file is touched.

Keep the Activity, Fragment, router, and WebViewClient as wiring layers. Separate responsibilities into focused modules:

- DOM/page adapter
- versioned protocol and manifest
- canonical native model
- MCP action and receipt handling
- native presentation and stable accessibility semantics
- session-state storage and recovery
- acceptance scripts and tests

Do not put vendor selectors, native rendering, session persistence, and command dispatch into one large file.

## 3. Build a Capability Matrix

Track implementation and acceptance independently for every capability:

- `code_status`: `implemented`, `partial`, `official_fallback`, or `missing`
- `verification_status`: `offline_verified`, `device_verified`, `user_action_required`, `deferred`, or `failed`
- `code_gap`: exact missing implementation, or empty
- `verification_gap`: exact missing evidence, or empty

Do not generate code merely to make a verification gap disappear. Do not call a capability complete because its code compiles. Read [references/acceptance-matrix.md](references/acceptance-matrix.md) when planning or recording a batch.

## 4. Use the Canonical Integration Chain

Implement the flow in this order:

1. Read only public, user-visible DOM state from the official page.
2. Convert page state through a versioned adapter into a versioned semantic manifest.
3. Convert the manifest into canonical native models.
4. Render native UI with stable `contentDescription`, test tags, and semantic control IDs.
5. Dispatch MCP/ADB actions through canonical commands and return structured receipts.
6. Fall back to the official WebView for unknown, sensitive, or newly changed functionality.

Keep the adapter limited to transformation and interaction with visible page controls. Do not reverse-engineer private credentials or make the native UI depend on undocumented private APIs.

Version the page adapter and protocol together. Prefer roles, labels, state attributes, and explicit test hooks over fragile class-name or positional selectors. Represent unknown rich output structurally when possible; otherwise expose an explicit official-page fallback and never silently discard it.

Use desired-state commands for toggles and disclosures, for example `ensure_expanded` and `ensure_collapsed`. Reserve generic click commands for actions that are retry-safe. Make every MCP receipt report the requested state, observed state, result, and recovery hint.

## 5. Implement Code Gaps as One Coherent Batch

1. Audit the matrix and identify genuine code gaps.
2. Implement a coherent batch across adapter, protocol, model, UI, MCP, tests, and fallback.
3. Preserve specialized controls; do not add a generic command that duplicates an existing domain command.
4. Add safe session recovery using only allowlisted official origins and non-secret navigation state.
5. Keep private message text out of logs, screenshots, test fixtures, receipts, and summaries.

Prefer one complete vertical capability over scattered partial changes. Once the code matrix has no real gaps, stop adding speculative code and move to acceptance.

## 6. Verify Offline Before Using a Device

Run focused checks for each layer:

- JavaScript syntax and adapter contract tests
- protocol and model unit tests
- MCP action, receipt, and idempotency tests
- native presentation and accessibility semantics tests
- session recovery and origin allowlist tests
- repository source-size and document-modularity guards
- Android compile, unit test, lint checks required by the project, and APK build

Run long commands through the repository's logged-command wrapper with required output and a bounded timeout. Check the previous run record before restarting a command. A missing device means `verification_status=deferred`; it does not justify repeated ADB polling or a false device pass.

## 7. Commit, Push, and Publish Deliberately

Stage only planned files. Commit by coherent module, push with the repository's direct-network script, and rebase only after an actual non-fast-forward rejection. Run the exact finish command returned by preflight.

Publish one APK from one verified commit for an acceptance round. Record version name, version code, Git SHA, download source, and SHA-256. If repository policy permits release without a connected device, publish after offline verification and mark device evidence deferred.

## 8. Accept on One Device in Risk Batches

Use one installed APK and one device identity for the entire round. Treat USB and Wi-Fi transports for the same handset as one device.

Run batches in this order:

1. Safe batch: login-state boolean, bridge/protocol version, manifest schema, stable selectors, navigation, pagination metadata, display modes, fallback entry, back behavior, and recovery.
2. Reversible authorized batch: isolated test conversation, send, streaming response, model/tool selection, reversible toggles, and restoration to the prior view.
3. Sensitive supervised cases: login, password entry, Google account choice, file upload, microphone/voice, delete, logout, purchases, or any irreversible action. Run each case only with explicit user supervision.

Do not emit account identifiers or conversation content. Record only structural evidence such as state booleans, counts, semantic IDs, protocol versions, and pass/fail receipts.

## 9. Close with Evidence, Not Optimism

For each case, record capability, code status, verification status, risk class, APK version, Git SHA, stable control ID, expected result, observed structural result, and recovery result. Update the matrix only from current evidence.

Keep the broader Goal active when device or supervised verification remains. Final reporting must distinguish:

- business capability status
- local main status
- unrelated main-worktree files
- task-worktree status
- pushed commit and published APK
- deferred or failed acceptance cases
- repository `FINALIZABLE` result

Do not declare the whole module complete while any required capability is `missing`, `partial`, `failed`, or still requires mandatory device evidence.
