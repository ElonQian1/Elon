# 用户专属 ChatGPT 浏览器模块接入

PC 工作台 `/user-browser` 通过开放商业公共目录发现 `browser.chatgpt.session.launch`，执行服务器端 action confirmation，再调用商户模块运行时。主项目不接收 ChatGPT 密码、Cookie、Access Token 或私有 API 数据。

## 平台配置

1. 为可信模块服务器所属项目创建商户，并配置 HTTPS `merchant_runtime` Binding。
2. 使用 `contracts/open-commerce/user-browser-capability-v1.json` 的输入和输出 Schema 创建能力。
3. 保持 `kind=action`、`access_level=public`、`handler_type=merchant_runtime`、价格为 0。
4. 验证 Runtime Manifest 后发布商户目录。
5. 公共目录中只能保留一个活动的同名 `merchant_runtime` 能力；PC 入口遇到多个来源会失败关闭，不静默选择。

`public` 只表示所有已登录一龙用户都能请求该动作。实际浏览器档案所有权由主项目写入签名运行信封的 `requester_user_id` 决定，能力输入不能指定用户。每个用户仍需在模块服务器的远程 ChatGPT Web 页面自行登录本人账号。

## 验收

- 未登录用户无法启动。
- 未发布能力时页面显示不可用。
- 多个同名模块来源时拒绝启动。
- 用户勾选本人账号确认后才创建 action confirmation。
- 返回值必须是 `yilong.user_browser.launch.v1`、`target=chatgpt` 和单次 fragment 票据。
- 入口必须使用 HTTPS；仅本机开发允许 localhost/127.0.0.1 HTTP。
- 不同一龙用户启动后得到不同 `session_id` 和不同服务器档案目录。
