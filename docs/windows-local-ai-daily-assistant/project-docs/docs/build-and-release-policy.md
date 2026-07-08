# 构建与发布约束

最后更新：2026-07-09

## 一句话结论

MVP 先做到本机可运行和可演示，不急着做自动更新、代码签名和公开发布。

## 开发阶段验证

真实项目创建后必须补齐具体命令。建议至少包含：

```powershell
npm run typecheck
npm run lint
npm run test
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
```

## 发布阶段

公开发布前必须确认：

- 安装包来源可信。
- 不包含 API key。
- 不包含测试截图。
- 不包含本地数据库。
- 隐私说明在首次启动可见。
- 卸载或清理数据路径明确。

## 暂不做

MVP 暂不做：

- 自动更新。
- 企业分发。
- 静默安装。
- 开机自启默认开启。
- 后台服务注册。
