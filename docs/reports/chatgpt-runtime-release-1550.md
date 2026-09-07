# ChatGPT runtime release 1550 evidence

Date: 2026-09-08. Delivery and acceptance evidence only.

## Artifact

- Source: `3b5768341` (runtime fix `b30e31e21`).
- Version `1.1.1550`, code `1550`, adapter `294`, text-runtime module `9`.
- APK SHA-256:
  `a7f676b102077b07d13383871976f8bfce691ad96e07fdee0ae57ea3c47e9dbc`.
- Release build passed in 6m45s; the complete publisher passed in 448.9 seconds.
  Remote version/hash/size and whitelisted Xiaomi replacement update passed.
  Separate package readback confirmed 1550 and the ZIP text-runtime asset
  exactly matched committed source. No Cookie, data or login reset occurred.
- Log: `chatgpt-current-owner-release-20260908-20260908-015955-197`.
- This batch did not rerun Android JUnit or previously accepted voice cases.

## Production result

The APK MCP opened production `social_ai`, provider `chatgpt_web`, at a ready
guest homepage: zero native messages/draft/official draft, no streaming,
dictation or native voice. Exactly one fixed synthetic prompt was sent.
The native list reached two messages, including exactly one expected reply,
with an empty draft and streaming false. Observation took 8,925 ms including
MCP actions/polling; this is not first-token or direct transport latency.

The persistent native `social_chat.web_chat_last_send_command` receipt was:

```text
action=send_prompt; ok=true
[private_fallback:template_unavailable] [runtime_fallback:react_owner_path_limit]
```

Therefore the fallback production chat works but runtime/private sending is
still **not accepted**. Submit 9 narrows the failure to its combined depth/visit
budget (90/180); it does not reveal the actual full website depth. Do not mark
the capability complete or report this as a successful direct private POST.

The initial diagnostic queried `last_send_command`, a nonexistent field, and
filtered the web command list for the send. The actual production sender keeps
its last send under `web_chat_last_send_command`; reading that field recovered
the receipt without sending again. Main streaming is `web_chat_streaming`;
the nested web snapshot uses `streaming`.

The web command list separately contained eleven `list_model_options` failures
at roughly 9-second intervals, each reporting an invisible entry. This is
current evidence for reviewing failed-catalog retry scheduling, not proof of
the phone's thermal cause. Do not infer the provider lacks model capability.

## Restoration and remaining gaps

The test issued one ordinary new-chat command only after confirming its exact
two-message fixture, empty drafts and idle voice. The 30-second zero-message
wait expired; the actual new-chat receipt reported no entry found. It was not
retried and no forced reload, deletion or data reset was used. The app then
returned to `conversation_home`, with zero native draft, as independently read
back. Blank-chat restoration was **not** accepted; the guest test context may
remain. Review this guest-home reset path before another blank-fixture test.

Submit 10 is a follow-up source candidate, not inside 1550. It uses memoized
linked paths with 512-depth/1024-visit engineering limits, unchanged sibling
bounds and separate depth/visit failure codes. Three deep valid trees fail on
submit 9 and pass on submit 10; out-of-budget trees remain rejected. Integrated
verification: 239 pass, zero fail/cancel/skip, log
`runtime-deep-tree-integrated-20260908-20260908-021545-556`. Red baseline log:
`runtime-deep-tree-red-20260908-20260908-021440-990`.

The attachment and model contract modules still have their own older return-chain
lookups. Reuse a proven shared owner resolver rather than copying this fix into
each consumer. New-chat reset, failed model-discovery retries, the installed
submit-10 route, and other listed private scopes remain unaccepted. Google
stays last. The overall Goal remains active.
