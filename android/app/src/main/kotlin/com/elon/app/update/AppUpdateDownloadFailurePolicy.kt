package com.elon.app.update

import java.io.IOException

internal const val APP_UPDATE_MAX_AUTOMATIC_RETRIES = 4

internal enum class AppUpdateFailureDisposition {
    RETRY,
    FAIL,
}

internal class AppUpdateHttpException(val statusCode: Int) :
    IllegalStateException("下载源返回 HTTP $statusCode")

internal class AppUpdateIntegrityException(message: String) : IllegalStateException(message)

internal class AppUpdateBackgroundServiceException(cause: Throwable) :
    IllegalStateException("后台下载服务启动失败", cause)

internal fun classifyAppUpdateFailure(
    error: Throwable,
    runAttemptCount: Int,
): AppUpdateFailureDisposition {
    if (runAttemptCount >= APP_UPDATE_MAX_AUTOMATIC_RETRIES) {
        return AppUpdateFailureDisposition.FAIL
    }
    return when (error) {
        is AppUpdateBackgroundServiceException,
        is AppUpdateIntegrityException,
        -> AppUpdateFailureDisposition.FAIL

        is AppUpdateHttpException -> if (
            error.statusCode in setOf(408, 425, 429) || error.statusCode >= 500
        ) {
            AppUpdateFailureDisposition.RETRY
        } else {
            AppUpdateFailureDisposition.FAIL
        }

        is IOException -> AppUpdateFailureDisposition.RETRY
        else -> AppUpdateFailureDisposition.FAIL
    }
}

internal fun appUpdateFailureMessage(error: Throwable): String = when (error) {
    is AppUpdateBackgroundServiceException ->
        "系统后台下载服务未能启动，请重新打开一龙后重试"

    is AppUpdateIntegrityException -> "安装包校验失败，请重新下载"
    is AppUpdateHttpException -> if (
        error.statusCode in setOf(408, 425, 429) || error.statusCode >= 500
    ) {
        "服务器连接不稳定，已保留下载进度"
    } else {
        "下载地址暂不可用，请稍后重试"
    }

    is IOException -> "网络连接中断，已保留下载进度"
    else -> "下载未完成，请稍后重试"
}
