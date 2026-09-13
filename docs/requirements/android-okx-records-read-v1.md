---
version_status: current
reviewed_at: 2026-09-14
decision_status: accepted
implementation_status: in_progress
owner: quant-grid
---

# Android 欧易策略记录只读托管

承接个人合约网格商业 Goal G07/G21/G22，主 APK 原 Provider 新增版本化 records_v1，量化持有记录产品 UI。仅 Read 授权、账户/主子类型/代次/调用签名边界继续复用现有宿主；主项目不复制量化页面。

固定调用策略详情、grid/sub-orders（live/filled，每页50，按ordId游标）和 grid/positions（无分页、最多10条），只接受指定本人策略及 USDT SWAP 合约。请求前后复核实际账号，校验响应策略/合约、重复/次序/数量，并保留数据来源与观察时间。前台授权恢复不外传凭据，撤销/换号使迟到结果失效。

记录合同为量化仓库 docs/contracts/okx-host-records-v1.md。金额、数量、费用/返佣以规范字符串及原币种传递，缺失为null；成交委托累计信息不能冒充逐笔成交或配对收益，持仓数量不能冒充基础币金额。无真实交易写操作。

文件计划：OkxReadProtocol/Host/Provider 各新增约15～30行；独立 OkxRecordsPage/Projection 各不超过180行；两个测试文件各不超过250行，包含固定请求/字段、跨账号/代次/策略/合约、分页及空/重复/超限、旧V1/V2兼容。官方原签名发布、手机安装、实际账号和视觉分别验收；不以本地假数据当交易所证明。回滚无存储迁移，旧客户端继续原接口。
