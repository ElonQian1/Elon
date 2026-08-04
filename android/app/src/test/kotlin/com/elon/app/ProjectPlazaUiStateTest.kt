package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ProjectPlazaUiStateTest {
    @Test
    fun primaryActionFollowsMembershipAndJoinMode() {
        assertEquals(
            ProjectPlazaPrimaryAction(ProjectPlazaPrimaryActionKind.OPEN, "进入空间"),
            projectPlazaPrimaryAction(project("approval"), joined = true)
        )
        assertEquals(
            ProjectPlazaPrimaryAction(ProjectPlazaPrimaryActionKind.JOIN, "加入项目"),
            projectPlazaPrimaryAction(project("open"), joined = false)
        )
        assertEquals(
            ProjectPlazaPrimaryAction(ProjectPlazaPrimaryActionKind.REQUEST_JOIN, "申请加入"),
            projectPlazaPrimaryAction(project("approval"), joined = false)
        )
        assertEquals(
            ProjectPlazaPrimaryAction(ProjectPlazaPrimaryActionKind.OPEN, "进入体验"),
            projectPlazaPrimaryAction(project("readonly"), joined = false)
        )
        assertEquals(
            ProjectPlazaPrimaryAction(ProjectPlazaPrimaryActionKind.OPEN, "查看项目"),
            projectPlazaPrimaryAction(project("invite"), joined = false)
        )
    }

    @Test
    fun pendingAndBusyActionsCannotSubmitTwice() {
        val pending = projectPlazaPrimaryAction(project("approval"), joined = false, requestPending = true)
        assertEquals("申请已提交", pending.label)
        assertFalse(pending.enabled)

        val busy = projectPlazaPrimaryAction(project("open"), joined = false, busy = true)
        assertEquals("处理中…", busy.label)
        assertFalse(busy.enabled)
    }

    @Test
    fun approvalStatusTakesPriorityOverInstallability() {
        val approval = projectPlazaAccessStatus(
            project("approval").copy(latestApkUrl = "https://example.test/app.apk"),
            joined = false
        )
        assertEquals("需审批", approval.label)
        assertEquals(ProjectPlazaTone.DANGER, approval.tone)

        val joined = projectPlazaAccessStatus(project("approval"), joined = true)
        assertEquals("已加入", joined.label)
        assertEquals(ProjectPlazaTone.SUCCESS, joined.tone)
    }

    @Test
    fun buildStatusUsesShortStableLabels() {
        assertEquals("构建成功", projectPlazaBuildStatus("completed").label)
        assertEquals(ProjectPlazaTone.SUCCESS, projectPlazaBuildStatus("completed").tone)
        assertEquals("构建中", projectPlazaBuildStatus("in-progress").label)
        assertEquals("构建异常", projectPlazaBuildStatus("failed").label)
        assertEquals(ProjectPlazaTone.DANGER, projectPlazaBuildStatus("failed").tone)
        assertEquals("暂无构建", projectPlazaBuildStatus(null).label)
        assertTrue(projectPlazaBuildStatus("unknown").label.isNotBlank())
    }

    private fun project(joinMode: String) = StoreProject(
        id = "project-1",
        name = "项目一",
        description = "项目简介",
        template = "android",
        ownerAccount = "elon",
        memberCount = 12,
        isPublic = true,
        joinMode = joinMode,
        lastTaskStatus = null
    )
}
