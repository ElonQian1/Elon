---
status: current
reviewed_at: 2026-08-23
---

# Win 网页 AI 私有响应结构研究 Fixture V1

## 目标

为 ChatGPT 与 Google AI Mode 的富内容适配提供一条可重复、可审计的离线研究链路：开发者把本人已获许可的本机调试响应交给独立工具，工具只输出版本化结构形状与合成值模板；回归测试消费脱敏结果，不依赖生产 WebView 临时读取、日志或手工猜字段。

## 范围

- 仅新增 Win 网页 AI 的离线研究脚本、合成 fixture、契约测试和说明；不修改 PWA。
- 输入只来自开发者显式指定的本机 JSON、NDJSON 或 SSE 文件；工具不启动浏览器、不读取 WebView Profile、Cookie、Token、请求头或网络。
- 输出使用 `yilong.web-ai-response-shape.v1`：保留有界 JSON 字段路径、容器类型、标量类型和长度分桶，不保留字符串正文、数值、URL、域名、动态 ID 或原始帧。
- 凭证形字段、请求头形字段、Cookie、Authorization、Token、签名参数和身份字段在遍历前整棵子树丢弃；字段名本身命中敏感规则时也不写入输出。
- 输出包含输入字节数分桶、SHA-256 指纹和清洗统计，便于比较同一厂商结构是否变化；不得包含可反推出原始响应的片段。
- 现有 `yilong.authorized-provider-response.v1` 与生产授权清单保持不变；本工具的研究结果不能自动开启生产私有响应观察器。

## 验收标准

1. CLI 可离线读取 JSON、NDJSON 与 SSE `data:` 帧，输出确定性的 `yilong.web-ai-response-shape.v1` JSON。
2. 输出只含白名单元数据和有界路径形状；测试输入中的正文、URL、域名、Cookie、Authorization、Token、签名值、账号标识和数值均不会出现。
3. 超深、超宽、超大数组和循环/异常输入失败关闭或有界截断，输出显式 `truncated` 统计，不导致内存无界增长。
4. 同结构不同内容生成相同结构指纹；结构变化生成不同指纹，便于 adapter fixture 回归定位。
5. 提供纯合成的 ChatGPT finance 与 Google weather/来源结构样本，覆盖脱敏、SSE 解析、敏感子树丢弃和结构漂移检测。
6. `npm run test:user-browser`、Win Web AI 验证、前端 typecheck/lint/build 和源码体积检查通过。

## 非目标

- 不抓取、拦截或重放厂商网络请求，不调用私有接口，不绕过厂商登录或真人验证。
- 不提交或上传任何真实厂商原始响应、用户问题、回答正文、Cookie、Token、请求头、完整 URL 或账号标识。
- 不从研究 fixture 直接生成生产授权，也不替代官网回答节点作为在线视觉真相。
