# Release 1682 And Current Canvas Provider Boundary

## Published Candidate

- Source: `68aad9c37211c36ff94eb8ca336d7fcfd9b65897`, adapter 361.
- Normal production APK: **1.1.1682 (1682)**, 38.42 MB.
- APK SHA-256: `5db4896478696d6682fe1586ce95a1d89ad06ac0901f84e6a33cce0df575a83c`.
- `canvas-grouped-apk-20260912-112612-601` passed in 458 seconds;
  `assembleRelease` passed in 6m53s. Remote size/hash/version matched.
- Whitelisted Xiaomi replacement installation completed and the APK MCP
  independently reported version 1682. Wireless ADB, hardware identity,
  production `social_ai`, ChatGPT authentication and adapter 361 were verified.
- No Cookie, app data, login or independent proxy change. Publisher restored
  its temporary Gradle version injection. Its existing background cleanup
  `Branch` warning is separate from successful publication/installation;
  the registered task finish contract still owns final cleanup.

This installs the original Canvas edit/history/restore, AI/comment actions,
PDF/DOCX export and Markdown/code-source export batch. Installation does not
prove all these operations work against this account's current provider.
The prior source checks remain in the [text export report](chatgpt-canvas-text-export-20260912.md).

## Production Canvas Result

`canvas-original-production-1682-20260912-113804-229` reached the actual native
conversation entry after exactly one synthetic native send. The fresh send
receipt used `official_runtime_v1`; the synthetic user anchor and completed
answer were verified. Canvas listing succeeded with `canvas_ready`, returned
zero documents and displayed the native empty row.

The run stopped at `canvas_fixture_not_created`: **zero saves, zero restores,
zero new share links**. Original conversation and awake settings were restored.
Editor/save/history/restore/export/AI/comment acceptance was not reached.
An empty list is not an editor failure or proof that all Canvas APIs are gone.

A separate read of this exact synthetic conversation confirmed that the model
explicitly declined Canvas creation and offered Writing Blocks instead. No
additional Canvas creation prompt was sent. This is account/model evidence,
not a global capability decision inferred from missing DOM.

## Current Official Product Evidence

OpenAI's [Model Release Notes](https://help.openai.com/en/articles/9624314-model-release-notes),
read on 2026-09-12, state in the 2026-05-28 entry that Canvas is no longer
available in GPT-5.5 Instant/Thinking. Writing and coding move into writing/code
blocks; legacy-model Canvas access was limited until those models retire.
The same page lists o3 retirement on 2026-08-26 and GPT-4.5 on 2026-06-27.

Do not extend that named-model announcement into an unsupported assertion about
every model, existing document, account or workspace. It does corroborate why
repeatedly asking the current model to create a Canvas is not an acceptance
strategy. Current downloadable bundles still contain Canvas code for existing
documents; an exported function or successful mock is not evidence that the
current selected model can create a new document.

## Acceptance Rules From This Result

- Preserve existing original/shared Canvas readers and scoped private actions.
  The earlier accepted shared-source viewer is not reopened by this result.
- Stop automatic creation retries on this model. Resume creation only with
  new evidence of a supported model/account; do not force undocumented tools.
- Original mutations require an owned disposable Canvas. An arbitrary existing
  user document or public shared document is not a writable test fixture.
- Do not rebuild or modify transport code solely because this fixture cannot
  be created. Existing bundle exports, offline checks, provider availability
  and successful native operations are four different facts.
- Audit current Writing Blocks/code blocks separately using actual responses
  and current provider source. They are not Canvas with a renamed label.
- Record outstanding provider/sample boundaries without marking them completed
  or repeatedly re-running the same setup. Continue other actionable ChatGPT
  gaps; Google and thermal work retain the user's later priority.

## Writing Block Follow-up

The existing fixture conversation was reused for one Writing Block request,
not another Canvas creation. Several prior semantic preparation attempts sent
zero messages: collapsed input did not expose the editor/send control expected
by the helper. A temporary helper change did not resolve this and was removed;
no production input or voice behavior was changed to make a test pass.

The one actual request used the existing canonical main MCP sender, not a
verified native Send-button click. Its response-read attempt encountered
`context_history_unavailable`; the dispatched request was not replayed.
Read-only reconciliation found `message_window_start=1`: the temporary probe's
hard-coded offset 0 was outside the available window, not a provider outage.
Omitting that offset used the canonical current-window default and succeeded.
The exact synthetic second user turn had one completed assistant reply with
both requested lines; underscores in its Markdown were escaped and `parts`
was empty. This confirms ordinary returned text, **not** a reproduced structured
Writing Block protocol or its native editing/export operations. No such
completion claim is made. A plain two-line reply is not proof of a new block.

The transient probe restored navigation/awake settings but initially failed to
restore its unsent draft. That report's `restored=true` was navigation-only,
not a full restoration pass. The exact owned synthetic draft was subsequently
matched inside the native input container and cleared; no user draft was
removed. Future probe cleanup must verify draft state as well as navigation.
After the actual send and read-only reconciliation, native draft state was
empty and the original conversation was restored. No new public link, Canvas
save/restore, microphone operation or recording was performed.

## Next Bounded Work

Inspect a genuinely structured Writing Block's current wire shape and native
rendering before writing a new adapter; do not repeat this completed text probe.
Keep original Canvas write verification deferred until a suitable owned sample
exists. Do not repeat verified voice, subtitles, dictation, read-aloud, ordinary
directory, sharing, upload or download paths without regression evidence.
