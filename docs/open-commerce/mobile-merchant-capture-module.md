# 手机商户平台采集模块

## 产品边界

`cofficethinking` 是独立商户模块，不是主项目内嵌浏览器。它的 Android APK 为商户本人提供美团外卖、淘宝闪购和京东到家可见 WebView。账号密码、Cookie 和网页签名凭据只保存在该 APK 的 Android WebView 数据目录，不进入一龙主项目或商户服务器。

一龙只负责确定“哪个一龙用户、哪个商户节点、哪个商户实例”有权签发一次性绑定入口。正式数据流为：

1. 商户编辑者在开放商业工作台配置并验证该商户的 HTTPS `merchant_runtime`。
2. “手机平台账号”面板登记固定的 `merchant.mobile_capture.session.launch` action 能力。
3. 面板为当前一龙用户创建独立开发者 App 身份，并创建仅含该能力、30 天、最多 100 次的 Grant；不使用可被其他用户复用的全局 `pc-web` 授权。
4. 用户明确点击后，主项目执行服务端动作确认，再通过现有 HMAC 商户运行时协议调用模块服务器。
5. 模块服务器把 `requester_user_id`、商户 ID 和固定门店 ID 写入单次票据；主项目不接受客户端自报门店 ID。
6. 主项目校验返回协议、Android 包名、`cofficethinking` scheme、64 位票据、纯 HTTPS origin、交换地址和不超过 5 分钟的有效期。
7. 用户在安卓手机打开限定包名的 intent，子 APK 用票据换取本机设备令牌。令牌由 Android Keystore 加密；服务器只保存摘要。
8. 用户在三个独立 WebView 中自行登录并打开订单列表。子 APK 只上传白名单订单响应，不上传 Cookie、密码或输入框内容。

## 代码所有权

- 跨仓能力合同：`contracts/open-commerce/mobile-capture-capability-v1.json`
- 主项目协议校验：`pc-frontend/src/features/open-commerce/merchantMobileCaptureProtocol.js`
- 主项目商户 UI：`pc-frontend/src/features/open-commerce/MerchantMobileCapturePanel.tsx`
- 子项目运行时、设备和 WebView：`D:\rust\active-projects\cofficethinking`

主项目 UI 只编排已有开放商业 API，不新增第二套调用、授权或计量账本。外部平台响应转换、门店归属、订单幂等和会话健康继续由子项目的 `MobileCaptureManager` 与既有 adapter 管理。

## 增加平台或业务 API

新增平台不修改主项目面板。子项目需要：

1. 在 provider registry 声明登录域名、可导航域名、精确采集域名和 endpoint key。
2. adapter 把该 endpoint 的真实响应转换到一个明确领域模型。
3. manager 统一执行设备鉴权、门店归属、批量限制、幂等、错误状态和重试。
4. 在能力矩阵中增加 `orders.pull`、`inventory.pull`、`settlement.pull` 等真实能力；不同领域不得伪装成订单。
5. 增加真实响应形状、错误响应、跨店拒绝、重复入库和 WebView 域名失败关闭测试。

主项目只有在顶层绑定协议、包名或 capability key 升级时才需要改动；平台页面内部 API 变化只更新子项目 adapter 和注册表。

## 发布与验收

主项目发布前至少通过：

- `pc-frontend/scripts/test-merchant-mobile-capture.cjs`
- `npm run typecheck`
- 定向 ESLint
- `npm run test:open-commerce`
- `npm run build`

子项目发布前至少通过后端 mobile capture 测试、Web 管理端构建、Android 单元测试、lint、Debug/Release 构建和 APK 签名检查。

真实闭环必须由商户本人监督：配置对应商户运行时和 Grant，在安卓设备交换票据，分别登录三个平台，触发一条真实订单列表响应，再从子项目管理端核对 provider session、原始幂等事件和统一订单。不得清除 WebView Cookie，不得把测试登录扩大为代用户输入密码。

当前主项目代码完成协议和 UI 接入，不代表每个商户实例都已配置运行时密钥，也不代表三家外部平台长期稳定。未配置商户应显示“等待运行时”，运行时拒绝或页面改版应返回明确错误并保留人工官网入口。
