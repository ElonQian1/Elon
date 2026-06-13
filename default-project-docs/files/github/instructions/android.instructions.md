# Android 与 APK 任务

本文件用于 Android、APK、Gradle、移动端构建和发布相关任务。

## 修改前

- 确认改动属于应用自身、用户子项目还是构建脚本。
- 阅读现有 Activity、Fragment、ViewModel、XML、主题和资源命名约定。
- UI 改动要同时考虑手机尺寸、触控区域、空态、加载态和错误态。

## 构建与验证

- 优先使用项目已有 Gradle wrapper。
- 选择与任务匹配的验证命令，例如 Kotlin 编译、单元测试、debug 构建或 release 发布脚本。
- Gradle 下载慢、缓存损坏或环境缺失时，先诊断依赖源、JDK、Android SDK 和缓存路径。

## 发布

- 可安装 APK 发布必须使用项目指定脚本和签名配置。
- 不在源码、日志或回复中泄露签名密钥、token 或服务器凭据。
- 发布后要验证版本信息、下载地址或应用内可见结果。
