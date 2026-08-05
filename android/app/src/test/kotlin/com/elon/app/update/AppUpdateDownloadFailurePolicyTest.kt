package com.elon.app.update

import java.io.IOException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class AppUpdateDownloadFailurePolicyTest {
    @Test
    fun networkAndTemporaryServerFailuresRetryWithinBudget() {
        assertEquals(
            AppUpdateFailureDisposition.RETRY,
            classifyAppUpdateFailure(IOException("connection reset"), runAttemptCount = 0),
        )
        assertEquals(
            AppUpdateFailureDisposition.RETRY,
            classifyAppUpdateFailure(AppUpdateHttpException(503), runAttemptCount = 2),
        )
        assertTrue(appUpdateFailureMessage(IOException()).contains("已保留下载进度"))
    }

    @Test
    fun automaticRetryStopsAtBudgetAndPermanentFailuresFailImmediately() {
        assertEquals(
            AppUpdateFailureDisposition.FAIL,
            classifyAppUpdateFailure(
                IOException("offline"),
                runAttemptCount = APP_UPDATE_MAX_AUTOMATIC_RETRIES,
            ),
        )
        assertEquals(
            AppUpdateFailureDisposition.FAIL,
            classifyAppUpdateFailure(AppUpdateHttpException(404), runAttemptCount = 0),
        )
        assertEquals(
            AppUpdateFailureDisposition.FAIL,
            classifyAppUpdateFailure(
                AppUpdateBackgroundServiceException(SecurityException("denied")),
                runAttemptCount = 0,
            ),
        )
    }
}
