# Windows 客户端 Defender 误报处理

## 现象

Windows 安全中心可能把一龙 Win 端报告为 `Behavior:Win32/Persistence.A!ml`，并隔离 `%LOCALAPPDATA%\ElonNode\一龙开发平台.exe`。这不是网页端访问地址被普通网络拦截，而是 Defender 的机器学习规则认为客户端安装和修复行为像“持久化”。

## 用户侧处理

如果用户确认下载来源是官方一龙地址，可以在 Windows 安全中心的保护历史记录里只还原或允许这一个 `一龙开发平台.exe` 文件，然后重新打开 `http://127.0.0.1:7799/api/status` 检查本机接口。

不要默认建议用户把整个 `%LOCALAPPDATA%\ElonNode` 目录加入白名单；目录白名单会扩大风险面。

## 产品侧处理原则

- 安装和修复不默认开启开机自启动。
- 开机自启动必须由用户在 PC 工作台手动点击开启。
- 修复客户端入口只恢复主程序、卸载程序、开始菜单和网页唤起协议。
- 发布清单写入 SHA256；客户端下载更新包后先校验哈希，再替换本地文件。
- 正式发行需要代码签名证书，并向 Microsoft Security Intelligence 提交误报样本。

## 客服排查口径

1. 确认报毒名称是否为 `Behavior:Win32/Persistence.A!ml`。
2. 确认受影响文件是否为 `%LOCALAPPDATA%\ElonNode\一龙开发平台.exe`。
3. 让用户只还原该文件，不白名单整个目录。
4. 让用户重新检测本机接口或导出诊断文件。
