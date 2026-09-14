# Fresh Tool Production Preflight

Status: preflight verified; independent Search/Image sending remains unverified.
Reused installed APK 1728 / adapter 395. No runtime source, APK or production
default changed in this batch. Writing Blocks completion remains governed by
[its feature record](../chatgpt-writing-blocks-native.md); its accepted editor
and export paths were not reimplemented or retested.

## Observations

- Initial directory requests ran after cached native history appeared but before
  the tool host was mounted. The same-document diagnostic reported
  `composer_tool_context:conversation_unavailable` with `composer_ready=false`.
  This is not an absent provider capability. A probe after restoring another
  conversation returned `not_observed`, which was not evidence about the failed
  document and must not be used to diagnose that request.
- The corrected, bounded host preflight passed in 13.6 seconds:
  `fresh-tools-host-preflight-20260914-184722-868`. It discovered eligible Search
  and restored the original conversation and screen-awake setting, sending no
  messages. Only tool-send acceptance waits for this host; no readiness condition
  was added to independent history or Writing Block reads.
- Native Search selection succeeded. The subsequent external UI runner failed
  before send because the real composer was collapsed. A read-only semantic
  inspection found the input absent and the Send button present/enabled. The
  page still contained our draft, zero matching user turns, zero new send
  receipts, and fresh-trial attempts remained zero. Thus this run does not prove
  independent HTTP dispatch, streaming or citations.
- The handset subsequently moved to another app. Further UI actions stopped on
  the foreground-package guard. The original conversation/tool/draft could not
  be restored then. The local fixture ledger remains pending, preventing the
  next run from blindly sending again; recovery must inspect this fixture first.
  The last screen-awake lease was restored. No cookies/data or proxy state changed.

## Acceptance Changes

`smoke-chatgpt-fresh-tool-dispatch.ps1` reuses the existing owned synthetic fixture,
requires native tool selection, one armed request, terminal history and matching
native output. It records uncertain writes before any send and refuses replay or
cleanup navigation until the result is known. Null catalogs are not counted as
options; command errors preserve safe enum codes and same-document diagnostics.

The external UI runner now has a separate preparation step: open only a unique
synthetic draft preview, verify the actual editable input, and only then allow a
later send step. Preparation cannot send. This source fix compiled, but its live
execution stopped on the foreground guard; it is not recorded as device-passed.
Send-action invocation and click acknowledgement are distinct report fields.
The PowerShell parser and 22 source contract checks passed, covering fixture
ownership, preparation before arming, ledger-before-send ordering, native-input
guards and pending-write cleanup fences. These checks do not replace device
acceptance of the new preparation step.

Remaining: resolve the owned pending fixture read-only, complete Search dispatch
and citations, then Image dispatch and native output. Promote each scope only
after that proof. Do not rerun accepted plain-text or Writing Block core cases.
