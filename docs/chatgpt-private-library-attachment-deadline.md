# Library attachment receipt deadline

## Current scope

Code implemented, offline verified and included in normal 1632/1634 Release builds.
The [1634 report](reports/chatgpt-library-append-1634.md) records installation and
the separately fixed first-file completion race. Slow mounted-file production
acceptance remains pending. This fixes the native Library "attach" lifecycle, not
upload speed, provider availability or thermal use. The existing mounted
preparation and download contracts remain unchanged.

## Defect

The native `WebChatLibraryAttachmentAction` stopped observing at 16 seconds.
`ChatGptWebObservedState` also assigned the default 20-second command deadline to
`attach_library_file`. The page operation can legitimately spend up to 10 seconds
confirming an existing conversation and another 12 seconds materializing its
mounted file. A valid result could therefore arrive after both consumers had
already reported an unconfirmed attachment and the browser no longer observed it.

The constituent requests were bounded, but the library operation itself had no
total deadline. Abort also depended on each awaited provider honoring its signal;
an uncooperative operation could keep the single attachment owner busy. These are
code/fixture findings, not a measured production failure on a cloud test file.

## One operation budget

- Library attachment v3 imposes a 24-second total deadline across both phases.
  One abort promise settles every duplicate receipt and releases the owner.
- A monotonic elapsed-time check also guards publication, including when the
  JavaScript timer is delayed. Late completion cannot stage a file or emit an
  attachment change after cancellation/deadline.
- A mounted selection remains consumed before its write. Timeout does not retry
  materialization, navigate, send text or delete any remote result.
- The native command allows 30 seconds and the UI observes for at most 31 seconds,
  leaving a delivery margin after page completion. Other command limits stay the
  same. The exact request ID and `library_attachment_associated` receipt still
  govern success; a terminal unknown result is not presented as confirmed.
- The UI preserves the draft and tells the user to inspect the conversation's
  attachments, not to repeatedly submit. Closing the Library only stops watching;
  it does not falsely claim to cancel a dispatched write.

`WebChatLibraryAttachmentReceiptPolicy` holds native outcome/timing rules. The
existing action class renders them; `ChatGptWebObservedState` only routes this
command to its specific budget. Sender v22 and adapter 315 activate the revised
library owner. No new protocol endpoint or parallel uploader was added.

## Evidence and next acceptance

At base `3d8d64f0b`, the new six-case JS suite had one pass and five failures:
four behavioral failures (owner still busy or late file published) plus the
missing native policy contract. The existing successful slow page operation
was retained as the positive case, not mislabeled as a failing transport.

After the correction, `library-deadline-related-20260910-134139-973` passed
**202/202**, zero skipped/cancelled. Cases include 20.5-second combined preparation,
uncooperative provider cancellation/deadline, duplicate commands, delayed timer,
ordinary-file late scope, existing attachment association/removal/send leases,
catalogue and mounted download compatibility. Production 109 JS assets and the
assembled bundle parsed successfully.

`library-receipt-jvm-20260910-134105-998` freshly compiled the production policy
and its JUnit test with cached Kotlin 2.0.21/Java 21: **8/8** passed. It covers
old time boundaries, exact late success, nonmatching receipt, terminal failure,
missing result, bounded observation and timing order. Source-wiring tests check
the actual UI/command consumers. This is **not a full Android build or an executed
Activity test**; full grouped builds are now recorded above, but slow mounted-file
Activity acceptance remains pending.

On a connected device with an authorized existing cloud fixture, use production
Library attach, observe the native card and return, and verify there is no early
16/20-second error or duplicate write. Reuse installed normal 1634 for its pending
checks unless a newer grouped artifact has actually been built and installed. Do not
rebuild or repeat already accepted ordinary file cases just to fill this gap.
