# ChatGPT runtime release 1547 evidence

Date: 2026-09-07. Evidence only; this report does not change product requirements.

## Delivery

- Extraction commit: `f7edf1473`; functional source: `9393d8792`.
- Release: `1.1.1547`, code `1547`, ChatGPT adapter `293`.
- APK SHA-256:
  `73e6a0c91d3f9c5fa334a16f8f83040f85351dd36ebeac1824fce575b39d60fb`.
- Release build succeeded in 7m23s. Publisher verified remote APK hash/size and
  version, then replacement-installed on the whitelisted Xiaomi hardware
  `e0d909c3`. Independent package readback confirmed `1547` / `1.1.1547`.
- Cookies, app data, account state and the independent proxy were not changed.
- The publisher's optional worktree cleanup reported a missing `Branch` property;
  this did not fail publication/install. The mandatory task finisher remains a
  separate cleanup/local-main check.

## Bounded production acceptance

Wireless ADB and the existing APK MCP were used, without coordinate tapping or
developer-page substitution. Production `social_ai` was authenticated, composer
ready, input empty, streaming false and voice idle. A dedicated new conversation
received one fixed synthetic prompt. No existing conversation was shared/deleted.

| Case | Observed result | Mark |
|---|---|---|
| Native text send | Exact synthetic reply reached the native list, but send receipt contained `private_fallback:template_unavailable` | Reply works; direct-private/runtime send not accepted |
| Native settled messages | Two expected synthetic messages plus an extra `正在思考` assistant bubble after streaming ended | Confirmed display regression; not accepted |
| Personal public share creation | Correlated `share_conversation` succeeded with `share_link_ready` and a validated official URL | `android_chatgpt_private_conversation_share_v1:personal_create` completed |
| Personal share list | Complete private list contained that exact newly created link | Included in list/revoke case |
| Exact public-share revocation | Consumed the list's selection ticket; receipt `share_link_revoked`; implementation requires complete readback without that ID | `android_chatgpt_private_conversation_shared_links_v1:personal_list_revoke` completed |
| Disposable current-conversation deletion | Correlated receipt `delete_server_acknowledged` after the share was revoked | `android_chatgpt_private_conversation_delete_v1:personal_current` completed |
| Restore | Original conversation restored successfully, followed by `show_conversation_home` | Passed before transport went offline |

The private share and delete handlers have no automatic DOM-write replay. These
are actual production-handler/MCP results, not proof that every rendered menu,
confirmation, Android share sheet or selection variant was tapped and verified.
Reuse these completed narrow cases unless a current regression is observed.

The send harness initially required exactly two native messages and timed out on
the extra status bubble. Reconciliation read the existing result without sending
again, accepted only the fixed synthetic texts and that known status string, and
recorded the display defect. It did not convert the failed exact-message assertion
or fallback receipt into a clean direct-send success.

After restore and package readback, wireless ADB changed to `offline` and USB was
absent. A final MCP home read could not complete. No further phone actions were
attempted; public-link revocation and disposable deletion were already confirmed.

## Next work

1. Diagnose the current official-runtime submission admission failure before
   claiming direct private text sending works on this website build.
2. Remove the settled thinking-status bubble at its real extraction/projection
   owner without discarding legitimate message content or hiding active progress.
3. Complete remaining model/tool/temporary-chat, gallery and attachment-scope
   acceptance from the existing implementations, not duplicate implementations.
4. Keep Google last and the overall Goal active. No latency, thermal or battery
   improvement was measured or claimed by this narrow release acceptance.

Offline binding/export evidence and regression counts are maintained in the
[runtime contract](../chatgpt-private-runtime-bindings.md). Earlier completed file
upload/download and empty-share-list cases remain in the
[previous grouped report](chatgpt-grouped-release-20260907.md).
