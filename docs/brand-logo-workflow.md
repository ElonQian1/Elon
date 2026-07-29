# 品牌 Logo 单一来源工作流

一龙自身品牌 Logo 的唯一源文件是 `assets/brand/logo.png`。Android、网页、PC 客户端和 Windows 安装程序不得再分别手工替换图标。

替换 Logo：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\replace-brand-logo.ps1 `
  -SourcePath "D:\path\new-logo.png"
```

脚本会保存原始 PNG，并生成：

- Android 五档 `ic_app_brand`、`ic_launcher`、`ic_launcher_round` 和 Adaptive Icon 前景；
- Tauri/Windows `icon.png` 与 `icon.ico`；
- 网页内嵌 `ic_app_brand.b64`；
- 一龙自身项目展示图标。

提交前执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\replace-brand-logo.ps1 -Check
```

源图必须是至少 192×192 的正方形 PNG。脚本只负责可由源图确定的位图工件；启动器背景色、网页布局和 CSS 不从像素猜测，需单独显式修改。
