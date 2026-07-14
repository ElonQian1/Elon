# Platform capability gap protocol

Use a platform gap only when the current Yilong tools cannot represent, render, measure, commit, or verify the requested UI outcome. A normal Android structure change is not a platform gap.

Required evidence:

- Original UI task ID and optional FitRun ID.
- Missing capability from the controlled capability names returned by `ui_check_capabilities`.
- The tool call or renderer evidence that failed.
- The smallest proposed platform change.
- A resume target describing the original UI step.

Lifecycle:

`AWAITING_APPROVAL -> APPROVED -> UPGRADING -> PUBLISHED -> RECHECKING -> RESUMED`

Only the PC workbench may move `AWAITING_APPROVAL` to `APPROVED`. Codex may start an approved upgrade, report the published commit/version, request a recheck, and resume the linked UI task. At most two platform upgrade rounds are allowed for one gap; otherwise move it to `HUMAN_REQUIRED`.
