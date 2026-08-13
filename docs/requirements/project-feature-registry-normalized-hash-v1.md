---
status: accepted
owner: project-governance
updated: 2026-08-13
---

# 功能注册表跨平台文本哈希 V1

## 目标

功能注册表验收脚本必须使用与服务端一致的文本哈希规则：将 CRLF 和 CR 规范化为 LF，再以无 BOM UTF-8 计算 SHA-256，避免同一需求文档在 Windows 与 Linux 上被误判为内容漂移。

## 范围

- 修正 `scripts/test-project-feature-registry-adoption.ps1` 的需求哈希计算。
- 保留对真实正文变化的失败关闭行为。
- 不修改现有需求正文或人工覆盖已登记哈希。

## 验收标准

1. Windows CRLF 检出文件与注册表中的规范化哈希一致。
2. 功能注册表采用测试通过。
3. CI 质量门禁登记检查通过。
