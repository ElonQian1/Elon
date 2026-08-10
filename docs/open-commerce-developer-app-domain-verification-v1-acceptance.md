# 开放商业开发者 App 主页域名控制证明 V1 验收

## 当前状态

`implementation_compiled_local_state_verified`

Rust `elon-server` 测试目标已编译；全新 SQLite 专项夹具已走通 challenge 签发、当前修订本地验证和资料编辑失效，并证明过期 challenge 会在白名单、DNS、TLS 或 HTTPS 处理前失败关闭且不记录网络尝试。测试没有发出真实网络请求，因此不构成真实域名控制证明。

## 已形成代码

- V152 保存域名验证主机、资料修订、challenge 摘要、期限、尝试时间、结果和稳定错误码。
- challenge 内容只在生成时返回一次；重复生成会替换旧 challenge。
- 平台只请求主页同源固定 `well-known` 地址，要求精确白名单并限制端口、重定向、超时、响应大小和文本格式。
- 资料编辑立即清除旧域名证明；资料提交要求当前修订已经验证。
- PC 可生成、复制 challenge 和固定地址，发起验证并查看状态。
- `server/src/open_commerce_developer_production_state_tests.rs` 仅验证本地状态依赖，真实网络安全边界仍由域名服务负责。
- `server/src/open_commerce_developer_domain_state_tests.rs` 验证过期 challenge 在出站处理前失败关闭；这不验证真实时钟漂移或网络。

## 统一回归必须验证

- V151 到 V152 升级、空 challenge 默认值和重复迁移；资料编辑失效的本地状态已通过专项测试。
- 明文仅返回一次、摘要匹配、换行裁剪、UTF-8、0/4096/4097 字节边界。
- HTTP、非 443、查询污染、重定向、超时、非白名单、错误状态和 DNS 变化。
- 重复生成、旧修订、并发验证、App 停用和已验证幂等读取；过期 challenge 的网络前失败关闭已通过专项测试。
- App 所有者、项目管理员、普通编辑者和非项目成员权限隔离。
- PC 状态、复制、禁用条件、错误提示和小屏布局。

## 仍未完成

- 多域名、子域委托、DNS TXT 证明和周期性重新验证。
- 声明式可撤销准入和生产凭据的本地状态联动已验证，但组织身份、工商资料仍未通过外部权威来源核验；公共网络权限仍默认关闭。
- 历史迁移、真实网络、权限和 UI 验证。

域名验证已接入公网地址解析与本次请求固定代码，但该网络安全边界仍未实际回归，统一网络验收见 `docs/open-commerce-outbound-public-address-pinning-v1-acceptance.md`。
