---
title: 节点插件 VFS Lock DMS Shared-Busy 动态切片权威 V1
status: current
reviewed_at: 2026-09-02
owners: node, security
design_status: design_frozen
implementation_status: q18_source_written_q19_requirement_frozen_source_not_started
verification_status: catalog_derivation_verified_source_review_only_actual_not_run
authority_scope: backend-a2-map-lock-dynamic-quotient-authority-v1
---

# Node Plugin VFS Lock DMS Shared-Busy Tranches Authority V1

## 1. Scope and status separation

本文从 [`Lock dynamic tranches authority`](node-plugin-vfs-lock-dynamic-tranches-authority.md) 拆出
`DmsSharedAcquire + BusyAfterKnownMutation + close-ok` 的成对纵切合同。父权威继续维护 Lock 全局聚合、
生产门和其它 q5+ family；[`Map/Lock dynamic quotient authority`](node-plugin-vfs-map-lock-dynamic-quotient-authority.md)
仍是登记功能的正式 requirement。

当前只有 q18 CreatedFirst 源码已写入；q19 ExistingFirst 在本文冻结需求，但在源码、测试、program inventory
和 implementation evidence 落位前仍是 `source_not_started/planned_missing`。设计、实现、静态验证和真实
Windows actual 四种状态不得互相推导。

## 2. Shared exact-family contract

每个 path 只承接 frozen Lock leaf 中对应的 `dms.<path>.shared-busy.close-ok` terminal：

- 8 个 `LockShared` 单槽与 36 个 `LockExclusive` 非空连续 range；
- `retention.succeeded.terminal.route-unknown` 与
  `retention.route-unknown-prior-quarantine.terminal.route-unknown` 各 44 个；
- 合计 88 members / 88 singleton normalized groups，shared/exclusive=`16+72`；
- matcher 必须全向量绑定合法 `first/count/mask`、`phase/fault_site=DmsSharedAcquire`、
  `timing=AtCall`、`occurrence=Natural`、`class=BusyAfterKnownMutation`、mutation known、lock certain、
  source failure disposition=`Returned`、DMS/file=`Released`、`cleanup_rewrite=false`；
- unsafe retention 后 terminal 必须是 `Quarantined`，callback/payload retained。

Close failure、DMS shared-error、joiner、其它 path、注入 Busy、same-handle overlap、tuple/case swap 或额外
lower attempt 均不得被这两个 program 吸收。

## 3. q18 CreatedFirst current source

`LockNativeAcquireCreatedFirstSharedBusyCloseSucceededV1` 精确承接
`dms.created-first.shared-busy.close-ok`。selector 为：

```text
initialization-{lock-shared|lock-exclusive}-first-{first}-count-{count}-created-first-shared-busy-close-succeeded-{retention-succeeded|retention-route-unknown-prior-quarantine}-terminal-route-unknown
```

未来受控 actual 必须在 fresh private root 上完成 cold CreatedFirst：target 真实取得 DMS exclusive、truncate、
exclusive unlock；同一 `FileId` 的 distinct holder HANDLE 再持有 `SHM_DMS_OFFSET` exclusive；target production
shared `LockFileEx` 恰一次返回 Contended，target `PinnedManagedSqliteFile::close` 真实成功。读取 terminal ledger
后才允许 holder 显式 unlock，随后 child report/exit，parent 清理 private root。

target lock attempt/success/contended=`2/1/1`、unlock=`1/1`、close=`1/1`；holder acquire/unlock=`1/1`，
两侧不得串账；requested-range native/local=`0/0`、managed=`1/0`、callback=`1/1`。`a2lockq18` 精确为
186 scalars，分区为 `25+5+25+8+14+43+15+18+1+18+6+4+3+1`。catalog 为 88 rows /
18,122 bytes，SHA-256=`4f78ff1678c93b1c06bad92e838423e4202598fd8e0b5b83f79cde0c528a07cd`。
这些均为 uncompiled/unrun source contract，不是 actual。

## 4. q19 ExistingFirst frozen requirement

`LockNativeAcquireExistingFirstSharedBusyCloseSucceededV1` 是 q18 的 path 对切，只承接
`dms.existing-first.shared-busy.close-ok`。selector 冻结为：

```text
initialization-{lock-shared|lock-exclusive}-first-{first}-count-{count}-existing-first-shared-busy-close-succeeded-{retention-succeeded|retention-route-unknown-prior-quarantine}-terminal-route-unknown
```

### 4.1 Real native sequence

q19 必须组合既有、互不替代的两段真实边界：

1. 先物理预创建并关闭 exact SHM file，绑定有序 precreation receipt=`[1,1,1,4,1,1,4,1]`；不得附着
   coordinator target，随后 cold attach 必须观察 `was_created=false`。
2. target 走正常 ExistingFirst DMS exclusive `LockFileEx`、真实 truncate、exclusive `UnlockFileEx`；同一
   exact `FileId` 的 distinct holder HANDLE 再真实持有 `SHM_DMS_OFFSET` exclusive。
3. target 的 production shared `LockFileEx` 恰一次返回真实 Contended；target close 必须真实成功。
4. `BusyAfterKnownMutation` 进入两种 unsafe quarantine/retention completion；terminal ledger 读取完成后 holder
   才 unlock，child 退出后 parent 才清理 private root。

不得用 synthetic Busy、同一 HANDLE、提前破坏 target HANDLE、`Drop`、holder cleanup 或 precreation file
identity swap 冒充上述顺序。q19 额外绑定 physical precreation 和 ExistingFirst observation，其余 target/holder、
requested-range 与 callback ledger 必须与 q18 同形且分账。

### 4.2 Wire and catalog

协议冻结为 `a2lockq19`、194 scalars：

```text
25 binding + 5 metadata + 8 physical-precreation + 25 cold + 8 callback +
14 after + 43 initialization/contention + 15 holder/close + 18 requested-lock +
1 pending + 18 terminal + 6 preemption + 4 registration + 3 route + 1 root
```

绝对区间为 `[0,25) [25,30) [30,38) [38,63) [63,71) [71,85) [85,128) [128,143)
[143,161) [161] [162,180) [180,186) [186,190) [190,193) [193]`。payload 必须拒绝缺失、重复、
换序或未关闭的 precreation receipt，拒绝 CreatedFirst、FileId/handle identity 错误、holder 未持锁、target
没有 production attempt、close receipt 失败或缺失、两侧 ledger 串账以及 receipt/case swap。

以 frozen Lock leaf 和 q18 已验证 canonical 顺序机械生成：先重建 q18 得到逐字节相等，再生成 q19；q19
目标 catalog 精确为 88 rows / 18,210 bytes / 89 LF / no BOM / trailing LF，SHA-256=
`eb318d91edbd0bbcd7e68ff626504a007a3f3c96d5eb60b965c9e362a421eee8`。88 个 case/full digest 各自唯一，
与 q1–q18 的 4,284 个 source-present member 零交集。该值是 q19 实现必须逐字节满足的 source target，
不是已写源码或 runtime evidence。

## 5. Planned aggregate and closed boundary

q19 完整进入 source inventory 后，计划聚合才可从 `4,284/4,284 present + 4,384/3,856 missing` 变为
`4,372/4,372 present + 4,296/3,768 missing`，total 保持 `8,668/8,140`；q12–q19=`704/704`，
initialization remaining=`2,728/2,200`。在此之前 current 聚合仍以 q18 为止。

q18/q19 的 descriptor `occurrence=Natural` 不等于 current natural actual。两批均不注册生产 VFS、不调用
production open，不创建 Runtime/Ready/Provider/Offer/Job/Lease，也不产生 market、settlement 或 funds effect。
Cargo、编译、Windows runtime、reviewed inventory、frozen quotient manifest 和 actual record 均未运行或不存在；
`Qlock=unknown`、coverage=`0/8668`、`WindowsDynamic=not_opened`。
