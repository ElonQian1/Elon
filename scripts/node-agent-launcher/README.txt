一龙 PC 节点客户端

普通用户用法：

1. 解压本压缩包。
2. 双击「一龙PC节点.exe」。
3. 浏览器打开「一龙 PC 工作台」后，用一龙账号登录。
4. 登录成功后，这台电脑会自动注册为 PC 节点并开始连接服务器。

「一龙PC节点.exe」会自动完成：

- 安装或修复到当前 Windows 用户目录：%LOCALAPPDATA%\ElonNode
- 创建桌面快捷方式「一龙PC节点」
- 注册当前用户登录后的开机自启
- 启动内部 elon-node-agent.exe
- 打开 PC 工作台，并在里面融合本机节点管理页
- 后续启动时自动更新内部节点程序

卸载：

- 在 PowerShell 或命令提示符运行：

  .\一龙PC节点.exe --uninstall

- 已经安装到本机后，也可以运行：

  PowerShell:
  & "$env:LOCALAPPDATA\ElonNode\一龙PC节点.exe" --uninstall

  cmd:
  "%LOCALAPPDATA%\ElonNode\一龙PC节点.exe" --uninstall

目录说明：

- 顶层只保留一个用户入口：「一龙PC节点.exe」。
- _internal 目录是程序内部文件，普通用户和 AI 代理都不需要直接运行里面的 exe。

高级配置：

- 普通用户不用编辑配置文件。
- 如需自定义服务器、端口、打开旧本地管理页或 TTS Worker，可复制 _internal\node-agent.env.example 为 _internal\node-agent.env 后修改。
