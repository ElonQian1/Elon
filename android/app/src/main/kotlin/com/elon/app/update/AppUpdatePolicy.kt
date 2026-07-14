package com.elon.app.update

internal data class AppUpdatePolicy(
    val selfUpdateEnabled: Boolean,
    val installerOwner: String,
    val disabledMessage: String?,
)

internal fun appUpdatePolicy(isDebugBuild: Boolean): AppUpdatePolicy =
    if (isDebugBuild) {
        AppUpdatePolicy(
            selfUpdateEnabled = false,
            installerOwner = "PC_NODE_ADB",
            disabledMessage = "UI 调试版由 PC 工作台自动构建和更新，无需在手机内安装正式版。",
        )
    } else {
        AppUpdatePolicy(
            selfUpdateEnabled = true,
            installerOwner = "ANDROID_PACKAGE_INSTALLER",
            disabledMessage = null,
        )
    }
