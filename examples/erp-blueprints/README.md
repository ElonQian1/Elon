# ERP 蓝图参考实例

`cofficethinking-instance.json` 与 `minimal-retail-instance.json` 使用同一 `official.erp` 蓝图和 `1.0.0` 内核版本，但采用不同主题与行业插件。咖啡店还声明了仅属于自身项目的烘焙参数私有扩展。

`release-1.1.0.json` 保留所有扩展点，因此两个实例都可通过兼容检查；升级和回滚前后，咖啡店的 `cofficethinking.roast_profile` 清单必须完全一致。

这些文件是机器合同样例，不代表平台已经执行源码复制、数据库迁移或生产部署。

`cofficethinking` 独立仓库同时保存了 `contracts/erp/instance-v1.json` 和对应说明，证明子项目可声明自己属于该蓝图生态，而无需把源码或数据库复制进平台仓库。
