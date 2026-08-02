# 开放商业 Schema 驱动调用表单 V1 验收

## 已验证

- PC 消费者沙盒不再对所有能力无条件发送空对象，而是先让用户填写商户发布的输入契约。
- 嵌套对象、可重复列表、文本、整数、数值、布尔值、空值、简单枚举、固定值和受支持格式可生成非技术表单。
- 数值文本会按契约转换为数值；本地日期时间会转换为 ISO 时间；无效 UUID、范围和列表数量会在调用前提示。
- 未声明默认值的可选字段默认省略；可选布尔值和固定值只有在用户明确选择后才提交，显式“否”不会与“未提供”混淆。
- 无法安全呈现的必填字段、复杂枚举、超深结构和超过 50 项的最低列表要求失败关闭，界面没有原始 JSON 编辑器。
- `action` 能力必须明确勾选确认；表单内容变化会清除旧确认。
- 成功调用会关闭当前表单，不能复用旧确认再次执行；失败时保留输入供用户修正。
- 同一份输入的重试复用一个幂等键；修改输入或切换能力后才生成新键。
- 调用仍复用已有授权、配额、Grant 预算、幂等、计量、审计和服务端输入契约校验。
- PC 明确说明技术服务当前仅记录计量、未扣真实资金，商户商品或服务金额以商户返回结果为准。
- 可执行模型测试、开放商业 PC 静态契约测试、定向 ESLint 和生产构建通过。

## 仍未完成

- 完整 JSON Schema、文件上传、条件字段、联合类型和任意附加属性表单。
- 由可信注册机构验证商户对 `query` 与 `action` 的分类。
- 真实订单、支付、退款、配送、外部平台通知和生产身份互认。
- 浏览器端确认的跨设备持久证据、商户签名、外部时间戳或链上证明。

## 验证入口

```powershell
npm --prefix pc-frontend run test:open-commerce
npm --prefix pc-frontend run build
Set-Location pc-frontend
npx eslint src/features/open-commerce/ConsumerCommerceSandbox.tsx src/features/open-commerce/CapabilityInvocationComposer.tsx src/features/open-commerce/CapabilitySchemaField.tsx src/features/open-commerce/capabilityInvocationSchema.ts --max-warnings 0
```
