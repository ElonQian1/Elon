# Independent Retry Native Acceptance

Capability: `android_chatgpt_fresh_regeneration_v1`, ordinary existing personal
plain-text retry only. The corrected native workflow passed on September 14.
This does not promote project, attachment, tool or feedback variants.

## Build And Device

- Canonical Release: `1.1.1726 / 1726`, adapter 394.
- Source: `9777989f795d32d67f4ea2ecf713003425b50450`.
- APK SHA-256: `d35db80400880a8de49d0c38ad6f32eb3862bd44e0d0d26c99101770451e815a`.
- Artifact size: 40,471,223 bytes. Public version metadata matched source,
  version and hash after publication.
- `retry-verified-branch-release-20260914-170634-208`: passed, 471.6 seconds.
  Canonical Android compilation/package/lint completed. Local autodeployment was
  disabled for this run; no global device configuration was changed.
- `retry-verified-branch-install-20260914-171608-797`: passed, 6.8 seconds.
  The hardware-pinned Xiaomi received `adb install -r` with one attempt and no
  force-stop, data clearing or forced launcher restart. Version readback matched.

The publisher warned about its optional LAN firewall setup and worktree
auto-cleanup (`Branch` property); neither warning changed APK publication.
Proxy, voice, writing/export and work-mode code were not changed by this fix.

## Prior Trial And Guarded Replacement

Before replacement, 1725 still reported `history_reconciliation_pending` with
`history=store_not_reconciled`, owner `owned/ready`, accepted true and pending
true. In the installed reconciler that history code is reached only after
authoritative history confirms the exact observed terminal reply; selection in
the website store remained unconfirmed. Thus the server outcome was known but
the end-to-end UI acceptance remained failed. Replacement does not count as
recovery and the old request was not replayed.

Installation guards required the exact controlled unsent draft, no attachment,
no dictation/native voice/read-aloud, no permission prompt, idle telephony and
no running microphone app-op. The new APK restored authenticated native UI and
the controlled draft. Its restored conversation was not the isolated retry
fixture, so the fixture-identity guard refused reuse before any write.

## One Actual Native Run

`retry-verified-branch-native-isolated-20260914-171904-347` ran the existing
production smoke with `NativeRetry`, `FreshHttp`, current native surface and
adapter 394. It created one isolated conversation, sent one synthetic seed and
clicked the real native regenerate action once. No additional user turn was
submitted by regeneration and no legacy runtime retry was used.

The structured `elon.chatgpt_web.regenerate_acceptance.v1` receipt reports:

- fresh HTTP confirmed, owned response reconciled and no pending transaction;
- native streaming observed, assistant content changed and reply completed;
- unsent draft and original user turn preserved;
- original conversation restored, production surface preserved;
- one seed, one retry, no private content or credentials emitted;
- no Cookie or application-data clearing.

The native projection's assistant identity did not change. Content did change,
and the independent transport's stricter observed-assistant/history ownership
check passed; a content-only change was not used to establish transport success.

The one-off wrapper then wrongly checked inherited `$LASTEXITCODE` after a
PowerShell script had returned normally and printed its passed marker. The
outer process is retained as failed; it is not relabeled or rerun. A separate
read-only audit parsed that exact receipt, required all success/cleanup fields,
and rechecked authenticated native adapter 394, idle streaming, empty draft,
trial disarmed and pending false. It passed as
`retry-verified-branch-receipt-audit-20260914-172159-419`, 2.8 seconds, with zero
additional message writes. Two earlier wrapper preflight failures also wrote
nothing (missing helper import and nonmatching restored fixture).

## Promotion

Source adapter 395 changes only the already-verified ordinary retry gate from
opt-in to opt-out; transaction module v24 replaces v23 on idle documents. The
existing admission scope, versioned preparation, command
ownership, one-write fence, authoritative history/branch reconciliation and
pre-dispatch compatibility path are unchanged. Explicit false still selects the
accepted runtime, and an uncertain dispatched write never falls back or replays.

The fixture now omits the enable flag in its normal case so the regression
actually exercises the default path. The explicit opt-out/trial test remains.
Publication of this default promotion is recorded separately from the 1726
opt-in acceptance; no repeated live regeneration is required for this gate-only
change.

`retry-default-promotion-contract-20260914-172519-902` passed 182 tests with
zero failures or skips. These cover default-on, explicit opt-out, sibling
reconciliation, retained uncertain writes, draft/account/branch changes,
stop/recovery and the still-gated project/temporary/tool send scopes.

## Default Release 1728

- `1.1.1728 / 1728`, adapter 395, transaction module v24.
- Source `4dee6cdc05dd7892da804a8203d622d667a7be8d` includes the unrelated
  mainline wallet update accepted by a non-conflicting rebase.
- APK SHA-256 `29650f939300d3fc354b3fb614175e662496e8bbcaeae16b8920aa0bb2b9f46c`,
  40,475,319 bytes; public manifest matched the canonical artifact.
- `retry-default-module-wiring-20260914-172625-786`: 28 tests passed.
- `retry-default-release-20260914-173038-291`: Release passed, 464.8 seconds.
- `retry-default-install-ready-20260914-174120-736`: passed, 7.4 seconds.
  The installer checked the exact artifact hash and its packaged module v24
  opt-out condition, then used one `-r` replacement without clearing data.
- Final MCP readback: authenticated native adapter 395, bridge ready, streaming
  false, empty draft, pending false and trial disarmed; zero additional writes.

Before the successful install, another delivery had installed build 1727 and
left the activity unbound. The first guard stopped before installation. After
one normal native-surface restore, a strict-mode/optional receipt-field mismatch
also stopped before installation; the one-off wrapper now confines the existing
polling helper to its usual non-strict scope. No application guard was relaxed.
The actual media/draft/request/telephony guards passed before replacement.

1728's unarmed default is proven by the packaged gate plus default-path unit
tests; the real production-button HTTP/stream/reconciliation evidence is the
1726 trial above. No second live generation was performed just for promotion.
