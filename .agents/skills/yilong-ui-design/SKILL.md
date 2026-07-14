---
name: yilong-ui-design
description: Use the Yilong PC UI workbench and real Android renderer for Android APK UI creation, visual styling, design/sketch/image matching, pixel-accurate restoration, Compose layout changes, or PWA/APK visual parity. Trigger when a developer asks Codex Desktop to create or modify visible Android UI, supplies a UI sketch or annotated screenshot, says spacing/color/size/typography/layout should change, or asks to fit the app to a design. Do not trigger for click behavior, networking, data, permissions, business logic, or nonvisual bugs unless the request also contains a visual UI change.
---

# Yilong UI design workflow

1. Use the `yilong_ui_live` MCP before repository-wide reading or source edits.
2. If the request includes local images, call `ui_import_desktop_task` with the user request and absolute image paths. Preserve whether each image is a clean target, an annotated request, a current screenshot, or a style reference.
3. Call `ui_get_design_task`, `ui_get_project_profile`, `ui_get_runtime_status`, then `ui_check_capabilities`.
4. For an existing screen, use the smallest runtime node/subtree and local FitRun tools. For a new screen, create the Preview-first skeleton, compile once, prepare the real renderer, then fit.
5. Let `ui_start_fit_run` perform numeric trials locally. Use Codex source edits only for structure, missing bindings, or unsupported properties.
6. Accept a candidate only after deterministic/source write-back and patch-free build verification pass.
7. If `ui_check_capabilities` or a FitRun proves the workbench itself lacks a required capability, call `ui_report_capability_gap`. Do not bypass the workbench with an untracked one-off workaround.
8. Never approve a platform upgrade yourself. Wait for the PC workbench gap state to become `APPROVED`, then make the smallest platform change, publish it through repository instructions, report the published commit/version with `ui_control_capability_gap`, recheck, and resume the original UI task.
9. Stop after the bounded upgrade/fit budgets or when the gap requires product or security judgment. Return the gap ID and the human decision needed.

Read [references/platform-gap.md](references/platform-gap.md) only after a platform capability gap is reported.
