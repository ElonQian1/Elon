# Android media-card return diagnosis, 2026-10-03

Status: reproduced on a registered Xiaomi; diagnostic tooling only. No production
navigation fix or new APK is claimed by this report. Times below are Asia/Shanghai.

## Scope and evidence

The user requested that we find two existing group messages and operate their
actual cards. The logged-in Win group MCP found both exact URLs within the latest
120 messages of the specified group. No group messages were sent or changed.

| Provider | Exact URL | Original message time | Card title |
| --- | --- | --- | --- |
| Bilibili | https://b23.tv/HOi2uUX | Oct 2, 12:31 | 别人邪恶、别人厉害 |
| Xiaohongshu | https://xhslink.cn/o/4Oatsa1J8RD | Oct 2, 22:17 | 帮助你做游戏的项目，将做游戏的门槛拉低 |

Installed versions: Yilong 1.1.1859 (1859), Bilibili 9.13.0 (9130500),
Xiaohongshu 9.49.0 (9490803). Douyin 40.6.0 and WeChat 8.0.78 were installed
but their complete return flows were not tested in this batch.

Identity was checked against the project device registry before operation.
The automated test clicked the exact accessibility description in the specified
production group, waited for the expected App, then sent Android system Back
at most twice, three seconds apart. It did not launch a substitute URL through
ADB. No screenshot or message-body export was needed.

## Results

| Stage | Bilibili | Xiaohongshu |
| --- | --- | --- |
| Before | Yilong MainActivity, task 3253 | Yilong MainActivity, task 3253 |
| Opened | StoryVideoActivity, task 3255 | DetailFeedActivity, task 3249 |
| System Back 1 | MainActivityV2, task 3255 | IndexActivityV2, task 3249 |
| System Back 2, after 3 seconds | Same Bilibili home | Same Xiaohongshu home |
| Returned to original Yilong task | No | No |

Bilibili system events:

- 18:24:48.794: IntentHandlerActivity is created in its new task 3255.
- 18:24:49.220: Bilibili creates MainActivityV2 underneath the content route.
- 18:24:49.223: MWebActivity is created; the router subsequently finishes.
- 18:24:50.577: StoryVideoActivity opens and the intermediate WebView finishes.
- 18:25:03.077: system Back finishes StoryVideoActivity.
- 18:25:03.094: Bilibili resumes its own MainActivityV2, not Yilong.

Xiaohongshu system events:

- 18:29:22.138: RouterPageActivity opens in its existing task 3249.
- 18:29:22.322: DetailFeedActivity opens; the router subsequently finishes.
- 18:29:33.742: system Back finishes the detail activity.
- 18:29:33.783: IndexActivityV2 resumes in the same Xiaohongshu task.

The second Back checks do not test a rapid double-Back exit gesture, and do not
prove the Apps can never exit. They confirm the reported failure to return
directly to Yilong after leaving the selected content.

Receipts are in the repository's local `ai-command-logs` Git metadata directory:

- `media-return-bili-history-bounded-20261003-182426-250.stdout.log`
- `media-return-xhs-live-20261003-182908-220.stdout.log`

After the tests, ADB explicitly brought the original Yilong task forward without
clearing App data or third-party history. This cleanup is not an automatic return
success and is not included in the test receipts.

## Cause and implementation boundary

The installed routers for Bilibili, Xiaohongshu and Douyin resolve as
`LAUNCH_SINGLE_TASK` with their own task affinity. A caller cannot assume its
Activity will be directly beneath the destination merely by omitting NEW_TASK.
The Bilibili trace also proves the issue is not only stale pre-existing history:
its new task creates a home activity before displaying the requested content.

[SocialMediaAppLauncher](../../android/app/src/main/kotlin/com/elon/app/sociallinks/SocialMediaAppLauncher.kt)
already unwraps an Activity context and omits NEW_TASK for Activity callers.
Repeating that change would not fix these observed routes. Changing launch flags,
inventing callback parameters, clearing foreign tasks or polling to steal focus
is not supported by this evidence.

The separate [WeChat fix](wechat-return-task-isolation-20260928.md) addressed
WeChat moving a shared caller task to the background. Its isolated launch does
not imply that other Apps implement the same return behavior. WeChat was not
retested here.

## Options for a subsequent product change

- Preserve the existing explicit App / in-app open choice. The in-app reader can
  control its own Back action when the provider actually supports in-app reading
  or playback; it cannot supply unsupported media by changing navigation alone.
- For App opening, evaluate a user-triggered "Return to Yilong" notification
  bound to the originating group and task. Respect notification permission, clear
  it after return, and test process recreation. This is an explicit shortcut,
  not interception of another App's Back gesture; it is not implemented here.
- A verified provider callback could support direct return, but no documented or
  experimentally confirmed callback for these exact installed routes was found.
- A PWA cannot set another native App's task stack. In particular, the previous
  iPhone PWA WeChat handoff avoids an intermediate browser page but does not
  guarantee return to the standalone PWA. A generic HTTPS callback can open Safari
  instead. No physical iPhone verification took place in this batch.

Platform references: [Android tasks](https://developer.android.com/guide/components/activities/tasks-and-back-stack),
[Apple Universal Links](https://developer.apple.com/library/archive/documentation/General/Conceptual/AppSearch/UniversalLinks.html).

## Reusable acceptance tool

[`test-social-media-return-device.ps1`](../../scripts/test-social-media-return-device.ps1)
requires an already-open target group, registered device, provider and exact card
accessibility description. It delegates the click to
[`SocialMediaReturnAcceptance.java`](../../scripts/android/SocialMediaReturnAcceptance.java),
checks foreground ownership before each Back, and reports only task/Activity
metadata. The receipt requires both the original package and task to match for
return success. It does not automatically restore or terminate external Apps.

The Java harness compiled and ran successfully in both real tests. The existing
semantic runner keeps its 40-second default; this bounded history search uses
its new optional 60-second timeout. No APK rebuild is required for the harness.
