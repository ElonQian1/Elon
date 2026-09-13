# Typed Writing Blocks 读取与保存

能力 ID：`android_chatgpt_typed_writing_block_save_v1`。

- `code_status=implemented`
- `verification_status=offline_verified`
- `production_status=published_1701_device_acceptance_pending`

## 本批修复

新版 `client_defined_widget / writing_block` 已有原生编辑和导出，但旧解析器只读取
`data.content`，忽略保存后的 `message.metadata.writing_blocks[id].content`，也不提供
云保存源身份。本批在基线 `8b494966a` 上用合成消息复现了“显示旧正文、保存源数为 0”。

现在读取合并保存后的内容与标题，空字符串同样有效；私有历史、流式消息和官网内存
投影共用一个解析器。支持完整、具有明确源消息及块 ID 的普通/个人项目写作块，复用
现有原生编辑器、导出、显式保存、单次 POST、回读确认与单节点更新，不增加 DOM 等待。

## 协议与归属

- `index` 是原始 `content_references` 数组位置，不是过滤后的卡片序号；不信任显示顺序。
- 保留已知 variant、标题、邮件 recipient/cc/bcc/subject 与原始 metadata；按官网规则
  接受 JSON 字符串及 URI 编码的 JSON 字符串 metadata，不猜测其他格式。
- 保存仍为 `/backend-api/conversation/message/writing-blocks`，不是 Canvas 或库文件接口。
- 回读后保存内容进入对应消息的 `writing_blocks` metadata，原始 reference 不被改写。
- 重复 ID、同时存在同 ID 文本包装/typed widget、目录超限、原始索引冲突、生成中、
  未知 variant、共享来源或库文件联动都不能授权写回。可显示的块保留本机副本能力。
- 账号、会话、项目、分支、源正文或待保存编辑变化时不继续写入；写入结果不确定时
  只读核对，不重发。不新增自动后台保存。

项目与普通会话复用现有归属检查。本批不开放临时聊天保存、共享会话副本保存、
库文件联动或未知 widget 形态；也不改变已验收的代码块导出行为。

## 源码证据

复用 `web_20260912` 保留的官网公开源码，只作 AST 检查，不执行下载的 bundle。
文件哈希与字段核对由 `scripts/test-chatgpt-writing-block-public-evidence.cjs` 钉住：

- `conversation-small-h1dtzoris1y9588z.js`：`Gza` 类型判断、`WBa` 原始引用索引、
  `Qza/Zza` metadata 编码规则。
- `a965fc59-fzrm5l4zirdbhwph.js`：`nl` ID 到引用索引映射、`al` 保存正文优先、
  `pc` metadata 合并与 `rc → nc` 的统一消息写回。
- `4813494d-gf2h57w5fiay19bd.js`：已知 variant 枚举。

这些证据支持客户端合同，不等于服务端和真机已验收；保留原接口 `updated_at` 并非
服务端 CAS 的限制，不能承诺跨设备同时编辑完全无竞态。

## 验证与后续

- `writing-widget-contract-20260913-145701-163`：63 项通过、0 失败/跳过。
- `writing-widget-evidence-20260913-145821-373`：68 项通过、0 失败/跳过，含实际保留官网源码。
- 新增测试串联真实解析器、归属策略、上下文和私有保存协调器，仅替换网络；覆盖普通/
  项目会话、空正文、CRLF、原始索引、邮件字段、身份及分支变化、本地编辑冲突、超时
  只读恢复、不重复 POST。旧 `:::writing` 与代码块测试一起通过。

本批随后随 [1701 集中包](reports/chatgpt-private-grouped-1701.md)构建、发布并安装。
手机锁屏，生产验收待解锁；需用受控 typed widget 在生产原生 UI 中编辑、
导出、显式保存一次并重开确认；同时验收尚未通过真机的项目保存。不得沿用 `1692`
普通 `:::writing` 的验收结论，或把本项登记为 `completed`。
