---
version_status: current
reviewed_at: 2026-09-21
---

# Windows Selected Group AI Replies

## Scope

Capability: `win_group_web_ai_selected_reply_v1`.
Implementation and fixture verification are complete; live Windows provider acceptance remains separate.

- Group message menu: AI reply for one message. Multi-select toolbar: AI analysis for the selected messages.
- Confirmation records the target group, message IDs/revisions, question and provider. ChatGPT is the initial choice; Google remains selectable.
- Results are automatically delivered through the existing server request state machine and membership/revision checks. Markdown and code text are retained.
- The task continues when navigating to another group. Its status names the original group; answers are never inserted into the currently viewed wrong group.
- Before dispatch, connection/login failures can be retried. After dispatch, checking/recovery never resends the prompt. A retained completed answer can retry delivery only; server completion is idempotent.
- Cancellation and account changes stop further processing and close only the isolated task document. Existing cookies, profiles, personal conversations and Android code are untouched.

## Integration

`group-ai/` owns the UI, state machine and transport port. Tauri runtime 13 adds a main-window-only `group_ai_web_session` command, with per-owner/provider/task labels and at most two retained documents per owner/provider. New frontend code rejects older hosts for this feature without disabling existing personal chat.

The shared WebView2 host was extracted without changing normal session paths. A task uses the account's existing WebView2 profile, but a fresh temporary ChatGPT document (or a fresh Google AI Mode document), not the user's personal current conversation. Its transcript is not written to the personal snapshot cache.

Group documents load the already-reviewed Android fresh-text sender and Sep 21 runtime binding. Command IDs pass the actual production base36 receipt ledger. A prompt-matched, completed assistant message plus completed private stream is accepted even if the page still holds its reconciliation lock. An active/partial stream is never posted.

The older Windows bootstrap test assumed Android's asset list still lived inside PageAdapter and that both platforms shipped identical capability sets. It now checks the canonical extracted Android asset manifest and the reviewed Windows subset. The added group bundle has explicit load-order tests. Normal Windows adapter generation remains unchanged.

## Verification

- 13 task state-machine tests: real private receipt admission, document isolation, partial/wrong-answer rejection, code formatting, selected revisions, duplicate starts, lost dispatch/completion responses, delivery-only retry, cancellation and account switching.
- Frontend typecheck, production build and touched-file ESLint passed.
- Windows host compiled and all 147 `local_ai_browser` Rust tests passed.
- Existing exchange WebView and local AI browser contracts passed after the shared-host extraction.
- Browser fixture exercised the real group React UI: single-message menu, two-message selection, confirmation, switching groups during generation, automatic original-group delivery and Markdown display. Screenshots at 1280 and 760 pixels were inspected. Provider and server transports were test doubles, not a real ChatGPT acceptance run.
- The broad legacy `test:user-browser` chain reaches an unrelated stale catalog assertion (36 Android parity entries versus an expected 17). That catalog and test were not changed by this task. Do not report the entire legacy chain as passed.
- The separate browser-tab contract still expects incognito mode in the unchanged internal browser host. Its existing profile behavior no longer has that flag; this unrelated baseline mismatch remains unmodified.

## Boundaries

- No new per-group ChatGPT project lifecycle, public-link continuation or native file/image transfer is claimed by this batch. It matches Android's accepted selected-message isolated-analysis scope.
- Selected attachment descriptions are included; attachment bytes are not silently uploaded. The confirmation warns when selected messages have attachments.
- Tasks survive frontend route changes, not a full desktop process restart. No secret or prompt journal is added to local storage.
- Windows real-account delivery and Google provider behavior still require live acceptance; Android's previous acceptance is not Windows evidence.
