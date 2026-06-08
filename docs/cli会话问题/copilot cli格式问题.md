让我深入追踪实际执行路径，找到还没修复的部分。● Search (grep)
  │ "isSingleScore|isSingleTotalGoals|isSingleHalfFull"
  │ (.\apps\user-app\src\modules\betting\domain\moreBetOptionsGenerator.ts)
  └ 3 lines found
● Search (grep)
  │ "MoreBetModal[^N]|MoreBetModal\b" in **/*.tsx (.\apps\user-app\src)
  └ 11 files found
● Read ModalsContainer.tsx
  │ .\apps\user-app\src\modules\betting\ui\BettingPage\ModalsContainer.tsx
  └ 215 lines read
발견! **`MoreBetModalNew` は実際に使われていない！** ModalsContainer は `MoreBetModal` を使っている。私の修正は間違ったファイルに当てられた。
● Search (grep)
  │ "export.*MoreBetModal" (.\apps\user-app\src\modules\betting\ui\components)
  └ 6 lines found
OK! `MoreBetModal` 其实是从 `MoreBetModalNew.tsx` 导出的——我的修改是对的。但问题可能在 `onSelectionChange` / `onMoreBetConfirm` 的处理链。让我追踪实际传递流程：
● Read MoreBetModalNew.tsx
  │ .\apps\user-app\src\modules\betting\ui\components\MoreBetModalNew.tsx
  └ L1:55 (55 lines read)
● Search (grep)
  │ "onMoreSelectionChange|onMoreBetConfirm|MoreBetSelection" (.\apps\user-app\s
  │ rc\modules\betting\ui\BettingPage\hooks\useBettingPageController.ts)
  └ 4 lines found
● Read useBettingPageController.ts
  │ .\apps\user-app\src\modules\betting\ui\BettingPage\hooks\useBettingPageContr
  │ oller.ts
  └ L554:630 (77 lines read)
● Read useBettingPageController.ts
  │ .\apps\user-app\src\modules\betting\ui\BettingPage\hooks\useBettingPageContr
  │ oller.ts
  └ L630:670 (41 lines read)
找到真正的 Bug！在 `handleMoreSelectionChange` 里，`score`/`totalGoals`/`halfFull` 的 `selectionData` 构建时**完全没有传 `isSingleOnly`**：
● Edit useBettingPageController.ts +3
  └ apps\user-app\src\modules\betting\ui\BettingPage\hooks\useBettingPageControl
    ler.ts
现在验证 TS 无错后提交部署：
● TypeScript check (shell)
  │ cd D:\rust\active-projects\fb2\apps\user-app && npx tsc --noEmit 2>&1
  └ 4 lines...
● Read AppUpdateChecker.tsx
  │ .\apps\user-app\src\shared\components\AppUpdateChecker.tsx
  └ L230:245 (16 lines read)
● Read AppUpdateChecker.tsx
  │ .\apps\user-app\src\shared\components\AppUpdateChecker.tsx
  └ L218:240 (22 lines read)
这是个已存在的文件语法错误（import 块跑到 `export default` 后面了），与本次修改无关。看看这个文件顶部的 import 是否完整：
● Read AppUpdateChecker.tsx
  │ .\apps\user-app\src\shared\components\AppUpdateChecker.tsx
  └ L1:30 (30 lines read)
文件顶部的 import 是完整的，第 230-239 行是重复的残留片段。删掉它：
● Edit AppUpdateChecker.tsx -11
  └ apps\user-app\src\shared\components\AppUpdateChecker.tsx
● Read AppUpdateChecker.tsx
  │ .\apps\user-app\src\shared\components\AppUpdateChecker.tsx
  └ L225:250 (25 lines read)
● Read AppUpdateChecker.tsx
  │ .\apps\user-app\src\shared\components\AppUpdateChecker.tsx
  └ L1:32 (32 lines read)
第 1-228 行是旧版本，第 230-422 行是新版本（更完整的）。保留第二个，删除第一个：
● Read AppUpdateChecker.tsx
  │ .\apps\user-app\src\shared\components\AppUpdateChecker.tsx
  └ L415:423 (8 lines read)
● Remove duplicate component definition (shell)
  │
  │ # 保留第二个完整定义（第230-422行），删除第一个（第23-228行）
  │ $file = "D:\rust\active-projects\fb2\apps\user-app\src\shared\components\A
  │ ppUpdateChecker.tsx"
  │ $lines = Get-Content $file…
  └ 9 lines...