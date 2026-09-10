# 游戏会话观测合同 V1

状态：协议实现与离线互操作验证；不是已部署登录服务。2026-09-11。

本包补齐第一方 ESK 游戏与主账号之间的会话观测编码和验签合同。它与 `sdk/asset-access` 的量化专用 `aat_` 只读授权独立，不修改原客户端、权限、身份映射或资金接口。

## 功能边界

- 主项目仍以 users/sessions 为身份根；游戏 BFF 在真实授权之后才能获得主账号引用。不得按邮箱、昵称、钱包地址或客户端自报 ID 合并用户。
- 支持六种受限动作：authenticate、inventory、order、quote、accept_quote、principal_withdraw。最后一种需要独立本金退出权限，redeem 不能代替它。
- `verifyObservation` 检查完整挑战、固定 issuer/key、授权范围、规范整数、签名及 10 秒观察期限；授予期限最多 15 分钟，精确到期即失效。仅观察时间允许最多 2 秒未来偏差，不给主授权起止时间宽限。
- 输出只是动作绑定的已验证观察，不是可直接作为游戏登录的 bearer、钱包绑定或付款授权。付款时仍需在主项目权威写事务中重新核验当前会话与预算，并与撤销排序。
- 本轮不实现授权码/PKCE、同意 UI、TLS 路由、服务认证、令牌持久化、真实签发或撤销。调用方不能把 SDK 验签通过称为生产登录完成。

## 编码

ID 为 1–128 位 ASCII `[A-Za-z0-9_.:-]`。摘要为 64 位小写 hex，Ed25519 签名为 128 位小写 hex。金额和时间为 0..i64::MAX 的规范十进制字符串，revision 为 1..u32::MAX。字段形状严格，未知字段拒绝；接入传输层还必须拒绝重复 JSON 键。

所有签名字节均为无空白、无 BOM、无尾换行的 UTF-8 JSON 字符串数组。算法为 SHA-256 和 Ed25519。

1. action_digest：`[kind, ...动作字段]`。order 字段 order_id；quote 字段 asset_id、policy_id；accept_quote 字段 quote_id、idempotency_key；principal_withdraw 字段 position_id、idempotency_key；其余无字段。
2. challenge_digest：`["esk.game.session.challenge.v1", main_issuer, "esk-game", "platform_recorded", nonce, credential_digest, action_digest]`。
3. 签名消息：`["esk.game.session.observation.v1", key_id, challenge_digest, main_user_id, main_session_id, grant_id, revision, not_before_ms, expires_at_ms, scopes_joined, observed_at_ms]`。
4. authorization_digest：`["esk.game.session.evidence.v1", SHA256(签名消息), signature_hex]` 的 SHA-256。

scopes 必须去重并按 play、inventory_read、redeem、principal_withdraw 顺序出现，包含 play。完整字段与动作白名单由 `src/contract.js` 严格验证。

## 后端调用

调用者必须从固定部署配置提供 mainIssuer、keyId、publicKeyHex；不得从待验证响应中学习公钥。expectedChallenge 必须来自本次 BFF 请求，nonce 每次从操作系统随机源取得，credential_digest 绑定仅后端持有的游戏受限 token。

```js
import { verifyObservation } from '@yilong/game-access-contract';
const observation = verifyObservation({
  expectedChallenge, observation: receivedObservation,
  mainIssuer: pinnedIssuer, keyId: pinnedKeyId,
  publicKeyHex: pinnedPublicKey, nowMs: Date.now().toString(),
});
```

签发方可复用 `observationMessage` 得到规范字节，但必须先在一致性读取中检查原主用户、主会话和授权当前状态。编码函数本身不执行这些数据库检查。本包不提供接受任意 JSON 就签发凭据的入口。

## 验证与交接

运行 `npm test`。测试内使用显式、公开可重建的 synthetic key；生产不得信任它。固定样例 `test/fixtures/session-v1.json` 只包含合成身份、签名、公钥和规范字节，无真实用户凭据。

游戏子项目 `ElonQian1/esk-game` 的独立 Rust 实现读取同一组固定字节和签名，验证 action/challenge/evidence 摘要与权限、过期和篡改边界。来源为当前会话批准的 Sui 首发、统一主账号、独立本金退出权限方向；没有改变主项目市场定价 ESK 的经济规则。

下一步：在主项目新增独立 game-access 授权码与观察路由，复用真实主会话校验和 TLS 传输边界，再由游戏 BFF 对接；不能扩大现有量化只读 token 来代替它。
