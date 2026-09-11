---
version_status: current
reviewed_at: 2026-09-11
implementation_status: partial
---

# 币安独立止盈止损 V3 交付证据

主APK1.1.1653(1653)、量化0.7.16(33)均已正式发布并原签名无线覆盖安装。主APK托管账号与固定V3命令；保护编辑、检查和最终确认UI归量化。Feature Registry保持implemented，实际交易、价格/CLEAR准备与完整视觉待验。

主源码`51291b2f6ab4e668491d7d7f216df8712db92605`，APK40,130,329字节，SHA-256 `2c4036f0a663d587f891d023327399288a60bb534543897226543463ee96733e`，原签名`f79567cf8a7e610e218aa4b7a1292be93a9623d9bc06a9bafbf47b030f99010c`。发布服务器回读摘要与版本一致，adb install -r成功且手机独立读回1653。

量化源码`a6c33397cc105289e0e2bb6e967f19d3173b8cf4`，APK SHA-256 `2f6bb1bde70443176168f2186b1d5a9a5e73157a49b7fafb018bc17ee3007966`，发布号`rel_2509cc6764314fd6a558b25844bf413f`；[PR60](https://github.com/ElonQian1/yilong-quant/pull/60)与[PR61](https://github.com/ElonQian1/yilong-quant/pull/61)包含实现和恢复修复。完整量化证据在子仓库`docs/delivery/grid-protection-20260911.md`。

## 已取得证据

- 主78项定向JUnit、26项管理/保护JS测试及其创建/读取回归通过；官方APK发布构建通过。量化251项Release JUnit、27项MCP、安全合同及精确源码Android CI通过。
- V3手机读取当前账号策略与保护，ROI/PNL完成只读准备；编辑失效旧确认后仍可继续编辑并再次准备，价格/CLEAR草稿控件及返回新鲜网格通过。没有金融写入。
- 新主APK与旧量化0.7.15完成V2四区导航、创建连接、管理详情/settings准备及返回网格兼容检查；V2结果不覆盖V3未验项。
- ROI由主端依据原始投入及调整额精确换算；准备及提交前重新核对账号、策略和保护基线，固定update-grid保留非目标字段。实际提交链仍需本人最终操作及回读验证。

## 尚未关闭

价格/CLEAR准备、实际写入及效果对账、换号和像素未由本批证明；追踪开关及限价独立编辑不在本次范围。模拟环境未独立验证，Paper服务发布标记不代表币安账户是模拟账户。Win研究版激活仍待既有重启确认，本次没有重启Win/Chrome。

原始日志与设备回执在受控工件库`grid-protection-v3`批次。官方发布完成后报告本机自动清理Branch属性缺失；统一收尾需另行报告，不把清理问题混为发布失败。此记录不宣告网格商业化总Goal完成。
