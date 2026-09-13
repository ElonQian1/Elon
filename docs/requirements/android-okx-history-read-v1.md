---
version_status: current
reviewed_at: 2026-09-14
decision_status: accepted
implementation_status: in_progress
---

# Android 欧易历史网格只读托管

承接量化已接受的个人网格商业目标和欧易托管合同。主 APK 为已授权量化提供固定历史分页读取，量化拥有入口、列表、详情和分页 UI。

新增 `history_v1(grant, after)`；`after` 首次为空串，其余为规范正整数。固定 GET `/api/v5/tradingBot/grid/orders-algo-history?algoOrdType=contract_grid&instType=SWAP&limit=50`，仅添加验证后的 after。结果使用 `yilong.okx_host_history.v1`，保留当前账号、主/子类型、代次、修订、新鲜期、bots，并显式返回 after、next_after、complete。

每页前后核对实际账号及平台会话；历史页与原读取共享网络互斥、限时/限量、原签名调用方和撤销保护。短非空页继续提供最小 ID，成功空页才 complete=true；重复、非法 ID、晚于游标记录和非 stopped 历史拒绝。stopped 映射已停止，不代表已经平仓。旧 capabilities/read/detail/revoke 合同保持兼容，新能力通过独立 capabilities_v2 声明。

修改限于 `exchange/okx/`（新增历史模块约 100 行，其余每文件新增低于 60 行）及领域测试，不扩建主 APK 网格 UI，不修改凭据域或交易能力。离线验证固定 GET、游标/数量/重复/失败、账号变化/撤销/迟到结果及原 V1 兼容；发布后再验量化双包调用，真实欧易资料和设备不可用时保留待验。
