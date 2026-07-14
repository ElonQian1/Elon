package com.elon.app.update

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class AppUpdatePolicyTest {
    @Test
    fun debugBuildIsManagedExclusivelyByPcNodeAdb() {
        val policy = appUpdatePolicy(isDebugBuild = true)

        assertFalse(policy.selfUpdateEnabled)
        assertEquals("PC_NODE_ADB", policy.installerOwner)
        assertTrue(policy.disabledMessage.orEmpty().contains("PC 工作台"))
    }

    @Test
    fun releaseBuildUsesAndroidPackageInstaller() {
        val policy = appUpdatePolicy(isDebugBuild = false)

        assertTrue(policy.selfUpdateEnabled)
        assertEquals("ANDROID_PACKAGE_INSTALLER", policy.installerOwner)
        assertNull(policy.disabledMessage)
    }
}
