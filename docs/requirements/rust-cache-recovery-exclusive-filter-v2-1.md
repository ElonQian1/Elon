# Rust Cache Missing-Workspace Exclusive Filter V2.1

## Problem

`gc -RecoverMissingWorkspaces -WorkspaceOnly` is the reviewed recovery path for reclaiming cache partitions whose recorded workspace no longer exists. Under a low disk watermark, the general LRU selector could reselect valid workspace partitions after the recovery selector ran.

## Requirement

Recovery mode must remain an exclusive filter regardless of disk watermarks, forced aging, retired domains, or old toolchain epochs. Only workspace-scoped partitions that have a valid marker, a missing recorded workspace, no active lock, and an age beyond the configured grace period may be selected.

## Acceptance Criteria

1. An existing workspace partition is preserved when the volume is below the warning watermark.
2. An eligible missing workspace partition remains selectable under the same low-disk condition.
3. Filtered workspace partitions report `missing-workspace-filter`.
4. Shared, quarantine, unknown, invalid-marker, recent, and actively locked partitions remain preserved.
5. The Rust cache platform regression suite covers the low-disk path.
6. A live dry-run reports no selected partition whose recorded workspace still exists before any C-drive apply.
