---
version_status: current
reviewed_at: 2026-09-26
implementation_status: in_progress
---

# ChatGPT Rspack Runtime Compatibility

## Failure And Scope

Installed Win `0.3.69+220e10975ebd7c2117fb876bcccc162eef7df365` reaches adapter
212 and a ready composer after the shared-bootstrap repairs. Sending still
reports `runtime_fallback:runtime_not_observed`. The observed website now uses
Rspack; the previous ESM profile and `getSharedProps`/`prepared_action` owners
are absent from the reviewed current public source. Replacing old export
aliases alone cannot repair this deployment.

Candidate `chatgpt_rspack_official_composer_transaction_v1` is **not completed**.
The new code uses the page's already-executed module cache and committed
AppScope, not a separately created store or copied Cookie/HTTP client. The
official `CUv.a` composer transaction retains integrity preparation, official
state updates and streaming. Old website profiles remain unchanged.

## Reviewed Public Sources

Source prefix: `https://chatgpt.com/cdn/assets/`. No login data is used to
download these assets; the inspection script parses but never executes them.

| Asset | SHA-256 | Contract |
| --- | --- | --- |
| `manifest-492fbfe6.js` | `7a287c1ec724ca0d81094ec8556c90de937f206a06ab2e13807bd7fce9b95648` | Observed entry/root/home/conversation routes |
| `633146.03cad12214.js` | `e1c8be6ea49317892798a70d98882449c009708fcd6698a49aaa38ef760c52e2` | Exported `__webpack_require__`, existing `c` cache |
| `238022.3eafa0ab02.js` | `e6af97f3203259bf34a122508c7e4b82fc5b3152db4d6e58e52fb8f24afd7e3e` | `OS` browser identity, `c3.a` AppScope token |
| `89983.093b3d59df.js` | `831e8add28249b4a023c744b0cf9ada3733c515786e736f757cf596886c198e0` | `X9.z` retains live scope in a React ref |
| `586656.107574cbd4.js` | `5487f345674cd8401e99bda5e1932303d04bdaee918faec643929a0f04d5e843` | `sAW` account/user/access atoms |
| `421899.a0eae5f5f4.js` | `54feea0ceab517f626c9a906a3a60b6736dfa2ae908f5b3a6aba5631a1a40c4c` | `LGwv` conversation state, `LN` stream service |
| `498514.f27755c4fc.js` | `96ab529b483124fa00509aabd9c38fc885413ffe97acbb2f9cb77e8e686035b4` | `vG` composer draft/model/upload atoms |
| `702636.84e9ad0ac6.js` | `11d152190df82eabe2cb8fef37a6d142eaccda03d766d6a34c8abb08180c8505` | `dk.submitChatGPTCompletion`, `PTZ` multimodal message construction |

The `CUv` composer transaction is in `934244.56bcd8ce43.js`; the runtime profile
admits only this exact asset, and requires it already executed in the same
document. It calls `dk.submitChatGPTCompletion`, which delegates to the
official completion service rather than duplicating the request protocol.

## Guards And Tests

- The loader imports only the observed, reviewed runtime URL. It never calls
  `require(id)`, executes downloaded module bodies or guesses a current alias.
- The committed composer path must resolve to exactly one scope node and one
  conversation ID. Account, user, auth generation, route, model, draft and
  parent node are checked before dispatch. No credential enters receipts.
- Explicit tools, projects, existing uploads and native attachment commands
  are not yet admitted to the new sender. In particular, an image request is
  never silently converted to text-only submission.
- Official invocation is single-flight. False/throw/timeout after invocation
  remains uncertain, blocks replay and does not trigger a second sender.
- 27 focused synthetic checks and 203 existing sender/attachment/orchestrator
  checks pass. The complete Win bootstrap assembly reaches ready.

## Remaining Acceptance

Win release `0.3.69+23d021cf4fb463794fbd8218dce86790cb473f1f` built successfully.
The official publisher reports local activation `activated`, terminal
`complete`, and remote outbox `synced`. MCP `update_and_restart` succeeded;
subsequent `win_control_status` confirms that exact release with both native
and frontend hosts available.

Read-only MCP capture on the installed release confirms adapter connection,
composer readiness and context readiness. It still reports zero projected
messages/directories and a failed `list_conversations` command. The current
official page is a project conversation, outside this candidate's admitted
ordinary-chat scope. This is evidence of remaining projection compatibility
work, not a missing website capability or a successful end-to-end repair.

After explicit confirmation, one fixed acceptance message was sent in a new
ordinary ChatGPT conversation. Installed `23d021cf4` reports `send_prompt`
successful; the official page shows the complete requested answer. No retry
or group post was issued. The original draft was retained in task memory for
restoration, not included in the acceptance message.

The same readback exposes the next failure: native `message_count` and
`assistant_message_count` remain zero while the official answer exists. The
old DOM reader searches `main` and old message/turn markers; the reviewed
Rspack source has no `data-message-author-role` marker. Thus website delivery
works for this sample, but native answer projection still fails.

Adapter 214 adds an ordinary-chat read path using the committed AppScope's
reviewed `LGwv.G` mapping and `LGwv.z` selected branch. It reuses the existing
rich history projector and preserves DOM projection when present. Read
admission retains account, document and conversation ownership checks but
does not wait for an idle/empty composer; send guards are unchanged. Sixteen
new focused tests plus the 27 previous Rspack tests pass.

## Installed Acceptance Result

Win `0.3.69+ca99127e16090e793be38e552d7366702993a21f` built and installed;
MCP confirms that exact identity, local activation `activated`, terminal
`complete`, and remote outbox `synced`. The existing acceptance answer was
read without sending another message. Native projection **failed**:
`win_act_d10daf33bb084263abfa0ba5b1daa028` still reports message counts zero.

The actual document after restart uses a newer deployment:
`manifest-06f8ceb1.js`, runtime `633146.6ed5d111e4.js`, and entry
`908190.d446cd6dfd.js`. Browser-research attached to the same production
WebView (`host_mode=ai_window`); it did not navigate, invoke requests or
capture API/identity origins. This deployment is not admitted by the prior
exact runtime profile, so the new projection cannot run. The earlier
single-send success does not validate sending on this newer deployment.

The public-source export contains 73 CDN assets. AST inspection reports
2,454 modules and four explicit parse gaps; the prior `LGwv` identifier was
not found in the successfully parsed subset. Do not assume identical export
aliases or silently treat malformed filtered sources as fully reviewed.
Source inspection now accepts an explicitly observed manifest basename and
reports per-file parse gaps; three fixture tests cover these changes.

Next work is to review the current deployment's identity, committed scope,
conversation and composer contracts, add a separately versioned profile,
and verify readback before requesting another write test. Do not mark this
candidate completed or enable an unreviewed alias substitution.

The original official conversation and its 809-character native draft were
restored and checked; no original draft, group message, image or second
acceptance message was sent. The transient sidebar search was cleared.

Upload association, image understanding, complete answer
delivery to the group and APK device acceptance remain **unverified**. A
passing offline test or HTTP 200 does not mark those workflows complete.

For repeatable public-source inspection use `scripts/research-chatgpt-rspack.cjs`
with an absolute evidence directory. `download` only downloads this recorded
manifest's entry and conversation assets; `index`, `source`, `snippets` and
`methods` inspect Rspack modules using Acorn. Do not commit raw public bundles,
private conversation snapshots or authentication data.
