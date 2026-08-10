# 消费者发现请求输入边界 V1

状态：已接受、已实现，并通过本地 Rust 与 SQLite 假数据验收。

## 决策

消费者发现由服务端统一规范化基础输入，不能依赖 PC 表单或第三方 App 自行保证格式：

- `query` 去除首尾空白；空值视为未设置，最多 200 个字符，控制字符失败关闭。
- `capability_key` 去除空白后复用开放商业能力键规则；空值视为未设置，非法值失败关闭。
- 空 `requester_app_id` 兼容为 `pc-web`；其余值复用 App ID 规则并继续校验当前用户所有权。
- `limit` 在进入目录、截断结果和生成排序凭证前统一要求为 1 至 50；0 或超过 50 失败关闭，不由不同入口各自静默改写。
- 目录文本查询把 `%`、`_` 和反斜杠作为普通字符转义，避免调用方意外扩大匹配范围。

候选目录窗口仍固定为当前运营方最多 100 个商户；`limit` 只控制最终返回数量，不改变排序候选集合。

## 非目标

- 不新增模糊分词、拼音、语义向量、地理距离或外部搜索引擎。
- 不改变授权、Grant、动作确认、计量或排序规则。
- 不证明目录完整、商户数据真实或结果属于全网最优。

## 实现入口

- `server/src/open_commerce_consumer.rs`
- `server/src/store/open_commerce_directory.rs`
- `docs/open-commerce-consumer-discovery-inputs-v1-acceptance.md`
