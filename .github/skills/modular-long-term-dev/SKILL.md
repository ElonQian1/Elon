---
name: modular-long-term-dev
description: Use when creating, modifying, reviewing, or planning code in any project where AI agents should avoid giant files, split code by responsibility, refactor safely, and coordinate parallel AI work without duplicating or conflicting changes.
---

# Modular Long-Term Dev

Use this skill before code changes, refactors, or reviews when maintainability matters. The goal is simple: do not let AI-assisted work create or worsen giant files. Keep changes modular, reviewable, and easy for other agents to continue.

## Core Rules

- Entry files assemble, route, or wire dependencies; they should not accumulate feature logic.
- New feature logic belongs in a focused module with a domain name, not in `utils`, `helpers`, or `common`.
- Prefer pure behavior-preserving extraction before changing behavior.
- Keep refactor commits separate from feature or bug-fix commits.
- In parallel AI work, divide ownership by module and sync frequently before editing, committing, and pushing.
- Never discard or rewrite another agent's work unless the user explicitly asks.

## File Size Guardrails

- Over 800 lines: check for an existing module before adding new logic.
- Over 1500 lines: only tiny fixes may stay; substantial logic should be extracted first.
- Over 120 lines in one function: consider extracting smaller functions or a focused helper.
- More than 5 touched files or multiple concerns: split into smaller tasks or commits.

These are guardrails, not excuses for mechanical churn. A small, obvious one-line fix can stay in place. A new workflow, protocol parser, UI panel, command runner, or storage path should not.

## Workflow

1. Sync and inspect:
   - Check git status and fetch remote changes.
   - Inspect recent commits touching the same large file.
   - If the workspace has unrelated changes, use a separate worktree or branch.

2. Map responsibilities:
   - Identify what the target file currently owns.
   - Name the responsibility being changed: UI composition, API handler, transport, parser, storage, prompt building, background job, validation, etc.
   - Look for an existing module with that responsibility.

3. Choose the change shape:
   - Tiny local fix: edit in place.
   - New behavior in a large file: extract a module first, then add behavior.
   - Existing mixed code block: move the whole block to a domain module without changing behavior.
   - Large mixed task: split into staged commits.

4. Implement narrowly:
   - Move code with minimal edits.
   - Add explicit imports, module declarations, exports, or route registration.
   - Keep names stable and domain-specific.
   - Do not rename unrelated modules during the same task.

5. Verify:
   - Run the smallest meaningful build, lint, or test command.
   - For pure extraction, compare behavior through existing tests or compile checks.
   - Check for untracked new files before committing.

6. Commit clearly:
   - Stage only task files, including newly created module files.
   - Use messages such as `refactor(server): extract project git module` or `refactor(android): split attachment composer from MainActivity`.
   - Mention the source file slimming when useful: `project_api.rs 3000 -> 2700 lines`.

## Decision Tree

```text
Need to add or change logic?
  |
  +-- Target file < 800 lines and change is local?
  |     -> edit in place if responsibility is clear.
  |
  +-- Target file >= 800 lines?
  |     -> find or create a domain module for the changed responsibility.
  |
  +-- Target file >= 1500 lines and change is not tiny?
  |     -> extract first, then change behavior in a separate step.
  |
  +-- Multiple responsibilities or >5 files?
        -> split task/commits before coding.
```

## Naming Guidance

Good module names:

- `project_git`
- `project_attachments`
- `codex_stream`
- `intent_router`
- `task_scheduler`
- `attachment_composer`
- `conversation_list`

Avoid vague names:

- `utils`
- `helpers`
- `common`
- `misc`
- `manager` without a domain

## Review Checklist

Use this checklist before finalizing a change:

- Did this task reduce or at least avoid increasing giant-file pressure?
- Is each new module named by domain responsibility?
- Are refactor and behavior changes separated?
- Are new files explicitly staged?
- Did the verification command cover the moved code?
- Did the agent sync with remote work before commit/push?
- Would another AI know where to continue the same feature tomorrow?
