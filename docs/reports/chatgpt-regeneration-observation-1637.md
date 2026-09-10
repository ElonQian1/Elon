# Native regeneration observation follow-up

Capability: `android_chatgpt_official_runtime_regeneration_v1`.
Not completed. This report distinguishes a real failure from a reproduced
observation defect; neither is a proof that the provider lacks regeneration.

## Normal 1637 device evidence

The existing synthetic probe was admitted on the authenticated native production
chat surface, adapter 321. It contained one completed user prompt and one
completed 47-character assistant reply. No initial prompt was resent.
The rendered native regenerate button dispatched once; its command returned
`official_runtime_v1:regenerate_unknown:timeout`.

Logged run: `native-regenerate-1637-20260910-183626-849`, terminal failure in
67.1 seconds. This is an unknown post-dispatch outcome, not evidence that no
server request ran. No automatic second regeneration was attempted.

## Narrow source correction

Contract/runtime v7, adapter 322, keeps mutation admission, the original parent,
new assistant identity, selected leaf, account and document checks. It adds no
POST, request template, DOM activation or automatic fallback after dispatch.

The existing 15-second deadline now makes one final readonly observation. A
fixture demonstrated v6 reporting unknown even when an owned completed stream's
tree had committed after its 20 short retries, without another stream event.
Timeouts otherwise include a closed reason rather than only `timeout`; this
allows the next phone attempt to distinguish a missing stream from a tree or
ownership mismatch. Raw exceptions and all private values stay out of receipts.

The new failing suite reproduced the missing reason and late-tree defect before
the correction. The final 137-case regeneration/model/current-runtime run passed
without skips or cancellations (`regenerate-observation-final-20260910-185540-996`).
It covers final deadline readback, closed failure reasons, late success after an
unknown outcome, replay fencing and production command receipts. The entire live
failure is not yet attributed to this late-tree race, and regeneration is not
marked accepted.

## Normal 1638 delivery

Source `525c99c49` was built, published and installed as normal `1.1.1638 (1638)`.
Gradle passed Release/lint in 7m22s; the publisher verified the remote APK hash
and size, then verified unattended replacement on the whitelisted Xiaomi.
An independent package-manager read confirmed code/name 1638/1.1.1638.
APK SHA-256:
`7f19849b0a3d189d716d8978f0aea832d4add855560c4cdc4daa73e97a9496b3`.

Log `regenerate-observation-322-release-20260910-185805-959` contains
`BUILD SUCCESSFUL`, `APK_RELEASE_STATUS=published` and
`APK_ADB_DEPLOY_STATUS=updated`. The outer log reader then threw a Windows
file-sharing error. Its state file still says running, but its recorded publisher
PID no longer exists and the actual installation is verified. Do not restart this
build based on that stale state file. Worktree auto-cleanup also warned about a
missing Branch property; repository cleanup is checked separately at task finish.

The real original conversation was restored before installation. Initially after update,
the phone was locked/asleep and the app was on conversation home with no bound
ChatGPT snapshot. No regeneration was attempted on 1638. A private, uncommitted
restore checkpoint is retained under Git-local research artifacts; it contains
only navigation/phase metadata, not message content or credentials. After unlock,
restore/open the same synthetic probe, capture structural protocol evidence and
run the existing native retry harness once. Do not resend the initial prompt or
claim this installation as a passing regeneration case. The following follow-up
supersedes the locked-device boundary, not the failed acceptance status.

## Normal 1638 unlocked follow-up

The production native retry button was dispatched once on the same isolated
synthetic probe, with no initial prompt resent. Run
`native-regenerate-1638-20260910-191608-620` failed in 34.3 seconds with
`official_runtime_v1:regenerate_unknown:timeout_owner_changed`. This isolates the
first rejected observation layer; it does not identify which identity component
changed. No automatic retry or fallback write followed.

A later readonly reopen found the same URL, one user and one completed assistant
turn. The assistant's 47-character text differed from the pre-click snapshot;
its native row ID was unchanged. This is evidence of changed reply content, not
a successful runtime receipt or verified provider UUID. The original view and
prior device stay-awake setting were restored. The old protocol capture had no
records, so it does not prove which HTTP request ran.

The v8 source batch adds component-level ownership reasons without weakening
the checks. Its failing fixture first reproduced all changes being collapsed to
`owner_changed`; 141 related tests pass after the change. The native acceptance
predicate is also corrected: stable display-row identity is not a provider UUID,
but changed text alone still cannot turn an unknown write into a pass. Optional
protocol capture and restoration are now ordered inside the runner rather than
outside its initialization/cleanup window. Production acceptance remains open.

## Normal 1639 delivery and fallback evidence

Source `5cfacf1dd`, adapter 323, was built, published and installed as normal
`1.1.1639 (1639)`. Release/lint passed in 7m47s; remote hash/size and unattended
replacement were verified. A subsequent package-manager read confirms 1639.
APK SHA-256:
`29c0ded97a7fcf6a628767647d46326f4f9bb11f3834802c778ab790ed2de754`.
Release run `regenerate-owner-323-release-20260910-193656-529` is terminal/passed.
Its separate worktree-cleanup warning does not invalidate the installed artifact.

Run `native-regenerate-1639-20260910-194723-146` clicked the rendered native retry
once on the same synthetic probe; no initial prompt was resent. It failed the
requested transport assertion in 30.2s. The command receipt was succeeded/ok with
an empty detail, the legacy DOM regeneration result in the adapter, not
`official_runtime_v1:regenerate_observed`. A readonly comparison confirms changed
47-character completed text in the same native row and same conversation.
This is fallback functionality, not successful private-runtime acceptance.

The bounded capture contains eleven metadata-only records, including a 200
stream response for `POST /backend-api/f/conversation` and a 200 stream-status
GET. No headers, content or credentials were emitted. The pre-run snapshot had
page generation 2; the post-retry capture had generation 3. The exact transition
time and reason the runtime was not used are not established. In particular,
1639 did not reproduce a component-level ownership failure; do not relax any
identity check based on this sample.

Later model-context/read commands timed out while native chat was not the
foreground app. Cached native state still reported ready, so it is not evidence
of a live ready document or a provider API failure. Phone manipulation stopped.
The runner restored its probe origin, but the outer original-user view has not
been confirmed restored after this run. Its private Git-local navigation
checkpoint remains pending native foreground. The prior stay-awake setting was
independently confirmed restored; no application data or cookies were cleared.

Source-only acceptance correction `f53960de6` now requires a fresh readonly page
ACK and unchanged document/turn immediately before native or direct retry.
Eighteen continuity cases, five reply-identity cases and three script parse checks
pass. This prevents a known stale-snapshot test path, without claiming to fix
production regeneration or invoking another write. Use the installed artifact
for the next controlled runtime-admission sample; no new APK is needed for this
runner change. Private-runtime regeneration remains unaccepted.
