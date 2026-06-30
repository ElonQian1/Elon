一龙 PC 节点客户端

普通用户用法：

1. 解压本压缩包。
2. 双击「一龙开发平台.exe」。
3. 浏览器打开「一龙 PC 工作台」后，用一龙账号登录。
4. 登录成功后，这台电脑会自动注册为 PC 节点并开始连接服务器。

「一龙开发平台.exe」会自动完成：

- 安装或修复到当前 Windows 用户目录：%LOCALAPPDATA%\ElonNode
- 创建桌面快捷方式「一龙开发平台」
- 创建开始菜单文件夹「一龙开发平台」，里面集中放启动、打开日志、导出诊断、检查更新、修复客户端和卸载入口
- 注册当前用户登录后的开机自启
- 用同一个「一龙开发平台.exe」启动后台节点 runtime
- 打开 PC 工作台，并在里面融合本机节点管理页
- 后续启动时自动更新客户端主程序

卸载：

- 双击「卸载一龙开发平台.exe」，或在 PowerShell / 命令提示符运行：

  .\一龙开发平台.exe --uninstall

- 已经安装到本机后，也可以运行：

  PowerShell:
  & "$env:LOCALAPPDATA\ElonNode\一龙开发平台.exe" --uninstall

  cmd:
  "%LOCALAPPDATA%\ElonNode\一龙开发平台.exe" --uninstall

维护与诊断：

- 普通用户可以从 Windows 开始菜单打开「一龙开发平台」文件夹，直接选择「打开运行日志」「导出诊断」「检查更新」「修复客户端」或「卸载一龙开发平台」。

- 导出脱敏诊断包（会自动打开诊断文件所在位置）：

  "%LOCALAPPDATA%\ElonNode\一龙开发平台.exe" --export-diagnostics

- 打开运行日志：

  "%LOCALAPPDATA%\ElonNode\一龙开发平台.exe" --open-logs

- 打开启动器日志：

  "%LOCALAPPDATA%\ElonNode\一龙开发平台.exe" --open-launcher-logs

- 后台检查更新：

  "%LOCALAPPDATA%\ElonNode\一龙开发平台.exe" --check-update

- 修复客户端入口：

  "%LOCALAPPDATA%\ElonNode\一龙开发平台.exe" --repair

目录说明：

- 顶层只保留两个用户可识别入口：「一龙开发平台.exe」和「卸载一龙开发平台.exe」。
- _internal 目录只放配置示例、版本信息和说明，不再放可运行的内部 agent exe。
- 启动、安装、更新、卸载入口日志在 %LOCALAPPDATA%\ElonNode\_internal\logs\client-launcher.jsonl；PC 工作台设置里可打开「启动器日志」。

高级配置：

- 普通用户不用编辑配置文件。
- 如需自定义服务器、端口、打开旧本地管理页或 TTS Worker，可复制 _internal\node-agent.env.example 为 _internal\node-agent.env 后修改。
