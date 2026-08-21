---
title: 外部矿池 service-managed market profile approval evidence ABI 权威
reviewed_at: 2026-08-21
status: current
owners: backend, security, ai-economy
design_status: design_frozen
design_scope: market_profile_inventory_approval_evidence_abi_v1
implementation_status: implementation_unwired
verification_status: design_review_only
---

# 外部矿池 service-managed market profile approval evidence ABI 权威

## 1. 唯一结论与状态边界

本页只冻结首个 external-pool service-managed market profile 的历史审批证据 canonical ABI、四眼 actor 边界与
compiled inventory 的确定性配对规则。它不选择任何价格、容量、SKU、runtime、ceiling、时限或 Adapter wire 载荷，
也不创建审批实例、API、表、migration、Profile current authority、Pool、Offer、Snapshot、Plan、Lease 或 Runner。

当前状态固定为：

```text
vertical_slice_architecture=design_frozen
market_profile_schema_abi=design_frozen
market_profile_inventory_approval_evidence_abi=design_frozen
initial_profile_approval_evidence=unselected
initial_profile_inventory=unselected
current_profile_authority=unconstructible
external_adapter_semantic_wire_profile=unselected
implementation=unwired/uncompiled/unrun
passed=0
failed=0
```

完整 Profile schema 与无环 review-material 见
[market profile authority](external-pool-service-managed-market-profile-authority.md)，纵切编排见
[V280 parent authority](external-pool-service-managed-admission-runner-authority.md)，本页验收见
[approval evidence acceptance](external-pool-service-managed-market-profile-approval-evidence-abi-acceptance.md)。

## 2. Purpose-specific 证据与不可替代边界

现有 platform reference price curve review、Adapter release review、onboarding review、activation-plan review 与测试 fixture
只能证明仓库已有 authenticated admin、四眼和 canonical receipt 模式。它们各自绑定不同 subject、schema、domain 与 effect，
不得凭 `review_digest`、`decision=approved` 或同一个 user ID 充当本页 evidence。

本页 evidence 只批准 exact market-profile `profile_review_material_digest`。它不证明 profile enabled/current、Provider eligible、
external Adapter wire 可解释、market writer 可调用或任何经济效果已经发生。普通 JSON、数据库字符串、环境变量、Provider owner、
market service actor、Job caller、fixture user 与 synthetic `local-owner` 都不是审批 authority。

Evidence 的 production consumption 固定为 server-compiled immutable artifact；V280 current constructor不得查询部署数据库寻找“latest review”。
本ABI不新增第二个V280 durable object。若未来要引入online approval table/API，必须新立authority并对父级唯一77列admission
durable inventory重新评审；不能只改表或以本页design-frozen状态暗中扩展durable对象。

V1 复用现有 IAM 的 `admin|owner` authenticated session，不发明 product/economy/security 三个 IAM 角色。三个治理责任被压成固定
`review_scope=product_economy_security_v1`；若以后要求三名独立领域审批人，必须升级 evidence ABI revision，不能在 V1 中暗加 actor。

## 3. 常量、domain 与 canonical 规则

常量逐字固定：

```text
APPROVAL_SCHEMA=compute_federation.external_pool_service_managed_market_profile_approval_evidence.v1
APPROVAL_ID_PREFIX=external_pool_market_profile_approval_v1_
APPROVAL_ID_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-MARKET-PROFILE-APPROVAL-ID-V1
APPROVAL_EVIDENCE_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-MARKET-PROFILE-APPROVAL-EVIDENCE-V1
APPROVAL_SOURCE_KIND=external_pool_service_managed_market_profile_approval
APPROVAL_REVISION=1
APPROVAL_REVIEW_SCOPE=product_economy_security_v1
APPROVAL_DECISION=approved
APPROVAL_CONFIRMATION=confirm_external_pool_service_managed_market_profile_approval
CANONICALIZATION=rfc8785_jcs
DIGEST_ALGORITHM=sha256
APPROVAL_MAX_JSON_BYTES=1048576
```

本页新 ID/digest 统一使用：

```text
SHA256(domain UTF-8 || 0x00 || RFC8785-JCS(value) UTF-8)
```

输入必须是 UTF-8 RFC8785/I-JSON exact bytes：拒绝 duplicate/unknown/missing key、float、非 safe integer、非 canonical number/string、
trailing bytes、额外 whitespace 与 parse 后重新 JCS 不逐字相等。所有 digest 为 64 lowercase hex；所有 user/profile ID 为去首尾空白、
无 control 的 1..160-byte identifier；时间为 canonical UTC nanos。

## 4. Envelope、ID 与 evidence digest

Envelope exact 7 keys：

```text
schema
approval_id
approval_revision
approval_digest
canonicalization
digest_algorithm
approval
```

`approval` exact 10 keys：

```text
profile_id
profile_revision
profile_review_material_digest
review_scope
decision
submitted_by_admin_user_id
submitted_at
approved_by_user_id
approved_at
confirmation
```

JSON type固定：top-level `approval_revision`与nested `profile_revision`是`1..=9007199254740991` safe integer，`approval`是object，
其余top-level/nested字段全是non-null string；ID material中的两个revision亦为同一safe integer。不得用数字字符串、null、array、
bool或object/string substitution。

`approval_revision=1`，其他 revision 拒绝。ID material exact 3 keys：

```text
profile_id
profile_revision
approval_revision
```

`approval_id=APPROVAL_ID_PREFIX || domain_digest(APPROVAL_ID_DOMAIN, id_material)`。ID 故意不含 review-material、actor 或时间：
Profile review-material 本身要先携带 `review_source.source_id=approval_id`，把 review-material 放回 ID preimage 会形成循环。

`approval_digest=domain_digest(APPROVAL_EVIDENCE_DOMAIN, envelope)`；计算时保留 `approval_digest` key 并将值置为空串，禁止删除
self-digest key、用 final `profile_digest` 替换 review-material、改用普通 serde JSON 或无 domain SHA-256。

## 5. 四眼 actor 与时间不变量

未来 purpose-specific issuance workflow 必须从两个独立 authenticated `admin|owner` session 注入 actor。Issuance输入不得从
HTTP body、CLI/env、profile JSON或预置compiled item取得actor；builder输出必须写入两次session-derived user ID。`submitted_at`与
`approved_at`分别由submit/approve step的server clock采样，输入也不得提交；compiled replay只回读stored bytes，不采新时钟。
固定不变量：

```text
submitted_by_admin_user_id != approved_by_user_id
submitted_at <= approved_at <= profile.valid_from
decision = approved
review_scope = product_economy_security_v1
confirmation = confirm_external_pool_service_managed_market_profile_approval
```

首个 production instance 必须拒绝 synthetic `local-owner`。`rejected` 与 `changes_requested` 可属于未来 issuance workflow 历史，
但不得进入 compiled positive inventory，也不得被转换为本页 V1 approved envelope。审批用户不得等于
`external_pool_service_managed_market` service actor；Provider owner 或 profile caller 身份也不能代替 authenticated reviewer。

本页只冻结 issuance owner 必须满足的输入/输出合同；当前没有该 workflow、sealed session actor token、API、CLI、DB row 或 receipt instance，
因此 canonical evidence 字符串本身不能冒充一次真实登录审批。

## 6. 无环构造与 Profile source projection

唯一构造顺序：

1. 由 `profile_revision` 派生 Profile ID，再由 Profile ID/revision 与 approval revision 派生 approval ID；
2. 选择全部待审批 Profile 载荷；submit/approve step分别从sealed session与server clock取得distinct actor/time，并先写
   `review_source={source_kind:APPROVAL_SOURCE_KIND,source_id:approval_id,source_revision:1,source_digest:"",approved_by_user_id,approved_at}`；
3. 保留完整 Profile 7/17-key envelope，仅把 top-level `profile_digest` 与 nested `review_source.source_digest` 同时置空，
   按 Profile authority 计算 `profile_review_material_digest`；
4. 把该 review-material digest、submitter/approver 与时间写入本页 envelope，计算 approval digest；
5. 将 approval digest 填回 `review_source.source_digest`，再计算 final Profile digest；
6. 对 canonical evidence/Profile bytes、三种 digest 与 source projection 做完整 readback，之后才允许形成 compiled inventory pair。

Profile `review_source` 必须逐字映射：

```text
source_kind=APPROVAL_SOURCE_KIND
source_id=evidence.approval_id
source_revision=evidence.approval_revision
source_digest=evidence.approval_digest
approved_by_user_id=evidence.approval.approved_by_user_id
approved_at=evidence.approval.approved_at
```

Pair identity还必须逐字满足：

```text
evidence.approval.profile_id = paired_profile_envelope.profile_id
evidence.approval.profile_revision = paired_profile_envelope.profile_revision
evidence.approval.profile_review_material_digest = recompute_review_material(paired_profile_envelope)
evidence.approval_id = derive_id(paired_profile_envelope.profile_id, paired_profile_envelope.profile_revision, evidence.approval_revision)
```

禁止把Profile A派生的approval ID、Profile B的review-material或第三个revision拼成一个表面各自可验的pair。

Evidence 禁止携带 final Profile digest。Final Profile digest 通过 source digest 传递绑定 evidence；evidence 又绑定双 blank
review-material，形成有向无环链，而不是相互引用的 fixed-point。

## 7. Compiled inventory selection 与 exact replay

计划中的 `profile_approval/catalog.rs` 只接受 checked-in canonical evidence bytes；Profile/evidence pair必须append-only retained，
禁止删除、改bytes/digest或重绑Profile。`policy.rs` 只消费经 owner-local deep audit 后的sealed pair。Current选择按 exact
`(approval_id,approval_revision)` 与 Profile source tuple 0/1 收敛：

- 0 项：返回 `current_profile_authority=unconstructible`；
- 1 项：重算 ID/review-material/evidence/final digest、四眼、时间与 source mapping 后才可进入 Profile current-selection；
- 多项、同 ID 不同 bytes/digest、同 Profile 不同 evidence、source tuple 分叉或 replay 漂移：失败关闭。

Historical audit必须按durable admission/profile保存的exact key命中恰好1个retained pair；0项、多项或bytes/digest漂移都是integrity
failure，不得降级成“current unconstructible”或latest fallback。Expired/revoked historical pair只允许形成private-field、non-Clone、
non-Serde的pure-audit authority，不能恢复current authority或授权新market write。

Replay 不采新时钟、不改 actor/time、不重铸 ID/digest，也不以 latest/current evidence 替换 historical exact pair。Evidence/Profile
canonical bytes必须逐字等于 compiled source；普通 DTO validation、`Clone` receipt 或 getter 集合不构成 current authority。

当前 catalog exact empty，不得加入 placeholder、全零 payload、fixture approver 或待填 JSON。完成本页只把 evidence ABI 标为
`design_frozen`；`initial_profile_approval_evidence` 与 `initial_profile_inventory` 继续 `unselected`。

## 8. 计划源码所有权与可见性

未来独占边界：

```text
server/src/compute_federation/external_pool_service_managed_admission/
  profile_approval/
    types.rs
    canonical.rs
    validation.rs
    catalog.rs
  policy.rs
```

Canonical DTO 可由 owner 内部 Serde 解析，但字段私有且 `deny_unknown_fields`；不得提供 raw public constructor。经验证 evidence 与
Profile/evidence pair 必须 private-field、non-Clone、non-Debug、non-Serde，无通用 `into_parts`。Purpose-specific issuance builder
未来必须消费两个 sealed authenticated actor tokens；catalog validator只能验证历史 canonical evidence，不能凭字符串重新铸造登录 authority。

Domain owner seam 最宽只能 `pub(crate)`，source-contract 锁唯一 catalog/policy caller；Store sibling、HTTP、env、Provider 或 fixture
不能直接构造 verified pair。本页不新增 module declaration、Rust 文件、migration、table、UDF、trigger、API 或 feature gate。

## 9. source-written 与产品审批门

进入 source-written 前必须同批提供：

1. Purpose-specific 双 session issuance workflow 与 actor provenance，不复用其他 subject receipt；
2. owner-local canonical/parser/deep validator、compiled catalog 与 Profile/evidence sealed pair；
3. exact empty/one/multi、actor substitution、digest cycle、canonical drift、replay conflict 与 historical pair tests；
4. 首个 byte-exact Profile payload、真实 evidence instance与其 source review；
5. external semantic wire profile selection、完整 V280 writer/Gateway/validator/Runner 纵切与恢复证据。

缺少任一项都不得打开 V254 fence、登记物理 migration、生成 current Profile authority、Pool/Offer/Snapshot/Plan/Lease/Runner 或经济效果。
本页只证明 approval evidence ABI 可实施，不证明任何产品载荷已经获批。
