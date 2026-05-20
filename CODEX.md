# CODEX Project Notes

## Project Root

Use this directory as the project root:

`D:\一龙\一龙参考库`

The parent directory `D:\一龙` contains the VS Code workspace file.

## Structure

- `server/`: Rust backend service.
- `android/`: Android client app.
- `docs/`: Architecture and workflow notes.
- `scripts/`: Setup, build, and deployment helper scripts.

## Common Commands

Backend:

```powershell
cd server
cargo check
cargo run
```

Android:

```powershell
cd android
.\gradlew.bat assembleDebug
```

## Notes For Future Codex Work

- Preserve existing user changes. Do not revert unrelated edits.
- Prefer small, focused changes with verification commands.
- This repository is initialized with `origin` set to `git@github.com:ElonQian1/Elon.git`.
- Several Chinese comments/docs may display incorrectly in some terminals because of encoding/codepage issues; inspect files carefully before rewriting text-heavy content.
