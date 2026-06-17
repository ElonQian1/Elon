一龙 PC 节点客户端

普通用户用法：

1. 解压本压缩包。
2. 双击「一龙PC节点.exe」。
3. 浏览器打开 http://127.0.0.1:7799/ 后，用一龙账号登录。
4. 登录成功后，这台电脑会自动注册为 PC 节点并开始连接服务器。

「一龙PC节点.exe」会自动完成：

- 安装或修复到当前 Windows 用户目录：%LOCALAPPDATA%\ElonNode
- 创建桌面快捷方式「一龙PC节点」
- 注册当前用户登录后的开机自启
- 启动内部 elon-node-agent.exe
- 打开本地管理页
- 后续启动时自动更新内部节点程序

卸载：

- 双击「卸载一龙PC节点.exe」。

目录说明：

- 顶层只保留用户入口和卸载入口。
- _internal 目录是程序内部文件，普通用户不需要打开。

高级配置：

- 普通用户不用编辑配置文件。
- 如需自定义服务器、端口或 TTS Worker，可复制 _internal\node-agent.env.example 为 _internal\node-agent.env 后修改。
