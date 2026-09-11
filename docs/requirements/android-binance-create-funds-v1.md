---
version_status: current
reviewed_at: 2026-09-12
decision_status: accepted
owner: quant-grid
---

# 创建可用资金的固定只读宿主接口

承接量化商业目标G06/G11；产品与精确合同以量化[创建资金V1](https://github.com/ElonQian1/yilong-quant/blob/main/docs/requirements/grid-create-funds-v1.md)为准。主APK只托管会话、固定读取和严格投影；没有新的主创建页，不转发凭据、 arbitrary URL/JS 或资金操作。

## 已捕获官网来源

已校验SHA-256的公开脚本`a349dcad.8aeb5444.js`（e6d7525e9af407ae79225b50d8cf78033b9421fa9c0b3f1e0d521cb7ee44e6d8）模块38200的QR导出调用`POST /bapi/futures/v2/private/future/user-data/getMaxWithdrawAmount`；模块3041传入`{assetName:"USDT"}`。76909根据组合保证金模式选择普通合约可转入资金或现货可用资金；21854另外与杠杆上限取小值，因此该金额不是最终允许的投资上限。

模块32365从28896的H0取得真实`data.enable`；已校验`main.fd963d90.js`（b5b6030c5def144957980b2aa11b05b6f675554003854aeb20977884929c51f9）对应`POST /bapi/futures/v1/private/future/portfolio/margin/get-user-basic`，无正文。组合保证金分支41837调用20768.IS，`POST /bapi/asset/v3/private/asset-service/asset/get-user-asset`正文`{}`，只使用USDT的free。

源码证明请求和消费关系，不证明当前账号响应。严格验证success/code、模式布尔值、非负精确金额及唯一USDT；缺失与失败不采用官网的0回退。普通/组合分支实际可用性单独验收，不以另一账号替代。允许精确字符串或安全整数；非整数JSON number在缺少精确保真响应解析时拒绝，不能将舍入后的余额冒充精确值。

## 实施与保护

新增独立JS资金读取工厂、Kotlin宿主及结果解析器，现有创建命令、WebView资产装载和适配器依赖注入只接线；单文件目标不超过200行。复用已授权同源请求/身份检查，不复制认证。新固定方法归当前创建操作，读取前后检查身份，代次/文档/取消/过期绑定；30秒预算、60秒新鲜度，未完成交易不借新读取规避原有核对。

专项JS和Kotlin测试覆盖两类来源、固定请求、精度/空/零/重复行、失败、换号、过期和迟到；量化严格合同/比例草稿测试与双包装机验收配套。正式准备与最终本人确认合同保持，执行校验仍由现有服务和交易所承担。没有真实交易、划转、接收资金或新的私有凭据存储。
