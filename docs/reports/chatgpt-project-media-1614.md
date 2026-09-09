# Project media and explicit membership reads

## Status

Capability: `android_chatgpt_private_new_project_attachment_upload_v1`.
Normal production APK `1.1.1614`, adapter 312, source
`70c1429ef9d2df563de2c2b86fa9f325dfa9eab1` was reused, not rebuilt for this test.
The existing [scope harness](chatgpt-project-media-acceptance-20260909.md) ran
against the selected, authenticated, empty project homepage on the authorized
Xiaomi. No input from a microphone was needed.

- Private three-file association succeeded (`private_attachment_associated`).
- Native send was accepted once (`official_runtime_v1:accepted`), with one user
  message and a completed attachment transaction. There was no DOM send click.
- The reply quoted the fixed TXT/PDF fixture content and correctly described
  the image's shapes/counts/colors. Expected answers were not in the prompt.
- Staging/removal/restaging stayed local before Send; no early upload occurred.
- The harness failed its final project-membership check, not upload or reading:
  the probe reported `membership_probe_unavailable`. Do not report the entire
  harness as a pass. Generation/file-reading took 93113 ms; total harness time
  was 122.9 seconds, without an execution timeout or stall.
- Restoration of the original project, empty draft/pending state and the prior
  awake setting passed. Synthetic remote test files/conversation may remain;
  no existing history or remote file was deleted.

Evidence: `chatgpt-project-media-1614-acceptance-20260909-234433-446` in Git's
command-log directory. Only structural results were emitted.

## Read-only follow-up

A fresh private project directory contained seven conversations, up from the
six seen in the preceding directory acceptance. Exactly one file-test candidate
was found. Opening it and reading the native context confirmed the exact fixed
test prompt, then a fresh `probe_conversation_project` receipt succeeded for
that same conversation/project. No second upload or send occurred.

The initial ad-hoc reply match was inconclusive. A bounded read-only recheck
used the native `friend` role and the same Markdown normalization as the media
harness. It confirmed the exact fixture prompt, three messages (one `user`, two
`friend`), 282 reply characters, no content truncation, no streaming, both exact
document markers and the correct image facts. Reopened fixture content therefore
passed without another upload/send; the earlier unmatched check is not evidence
of missing production content.

Immediate restore readback preceded asynchronous navigation and initially
reported false. Subsequent bounded MCP readback confirmed the original canonical
project homepage, zero messages/pending files, empty draft, ready bridge and
ready composer. This was not a failed restoration or a reason to navigate again.
After the final content recheck, restoration using the directory entry's path
was rejected. The original canonical project path was accepted; subsequent state
confirmed that homepage, zero messages, an empty draft and a ready composer.

## Source correction

The membership operation called `conversationPrefetchReady()`, which requires a
successful official history observation within two minutes. That is a background
prefetch heuristic, not a prerequisite for an explicit membership GET. The
related explicit file reader had already removed this dependency.

Transport v25 reuses that file-reader admission for membership: private opt-in,
available or acquirable identity, and no active failure cooldown. The request
still uses the same GET, `no-store`, response bounds, identity acquisition,
single-flight owner and failure handling. It neither refreshes the background
freshness timestamp nor changes prefetch, sends, attachments, voice or captions.
No endpoint or credential mechanism was added.

The new cold-membership test failed on v24 before the correction. The final
focused run passed the transport assertions plus the existing transport policy,
conversation-file and real directory-bridge suites. New cases cover cold and
expired-freshness queries, duplicate acquisition/read coalescing, auth rejection
and cooldown, missing identity, disabled private reads, and unchanged background
prefetch eligibility. A stale directory-factory version assertion in the older
file suite was aligned with the already released v13; behavior checks remain.

Logs: `membership-read-admission-baseline-20260909-235155-298` and
`membership-read-admission-final-20260909-235329-827`.
Implementation/verification: **source verified, grouped APK installed; targeted
expired-freshness device query remains pending**. Transport v25 was included in
the normally published and automatically installed 1616 and 1617 grouped APKs;
see [release evidence](chatgpt-image-gallery-1616.md).
Do not repeat the accepted three-file send. Next install should verify one
explicit read of the existing synthetic conversation after freshness expires.
Keep Google last. Do not repeat the accepted three-file send or publish another
APK just for this read-only follow-up.
