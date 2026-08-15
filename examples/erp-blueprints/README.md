# ERP 蓝图参考实例

`cofficethinking-instance.json` 与 `minimal-retail-instance.json` 使用同一 `official.erp` 蓝图和 `1.0.0` 内核版本，但采用不同主题与行业插件。咖啡店还声明了仅属于自身项目的烘焙参数私有扩展。

`release-1.1.0.json` 保留所有扩展点，因此两个实例都可通过兼容检查；升级和回滚前后，咖啡店的 `cofficethinking.roast_profile` 清单必须完全一致。

`release-1.2.0.json` 首次把 `@yilong/merchant-erp-kernel` 作为可机读运行时绑定写入发布清单，并加入采购入库能力。绑定只告诉物化代理应复用哪个公共内核；商户仍需提供自己的存储适配器、主题、私有插件和部署证据。

`upgrade-campaign.json` 展示升级前后公共配置快照、实例修订和商户采用证据。它不包含密钥、客户数据或私有扩展源码。

这些文件是机器合同样例，不代表平台已经执行源码复制、数据库迁移或生产部署。

`cofficethinking` 独立仓库同时保存了 `contracts/erp/instance-v1.json` 和对应说明，证明子项目可声明自己属于该蓝图生态，而无需把源码或数据库复制进平台仓库。
