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
- 用同一个「一龙PC节点.exe」启动后台节点 runtime
- 打开 PC 工作台，并在里面融合本机节点管理页
- 后续启动时自动更新客户端主程序

卸载：

- 双击「卸载一龙PC节点.exe」，或在 PowerShell / 命令提示符运行：

  .\一龙PC节点.exe --uninstall

- 已经安装到本机后，也可以运行：

  PowerShell:
  & "$env:LOCALAPPDATA\ElonNode\一龙PC节点.exe" --uninstall

  cmd:
  "%LOCALAPPDATA%\ElonNode\一龙PC节点.exe" --uninstall

目录说明：

- 顶层只保留两个用户可识别入口：「一龙PC节点.exe」和「卸载一龙PC节点.exe」。
- _internal 目录只放配置示例、版本信息和说明，不再放可运行的内部 agent exe。
- 启动、安装、更新、卸载入口日志在 %LOCALAPPDATA%\ElonNode\_internal\logs\client-launcher.jsonl；PC 工作台设置里可打开「启动器日志」。

高级配置：

- 普通用户不用编辑配置文件。
- 如需自定义服务器、端口、打开旧本地管理页或 TTS Worker，可复制 _internal\node-agent.env.example 为 _internal\node-agent.env 后修改。
