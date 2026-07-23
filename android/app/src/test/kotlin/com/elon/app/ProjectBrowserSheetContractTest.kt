package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import java.security.MessageDigest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ProjectBrowserSheetContractTest {
    @Test
    fun realProjectsAreGroupedAndSearchedWithoutChangingTheirIndexes() {
        val projects = listOf(
            project("personal", "个人长名称项目", 100, owner = "钱一龙"),
            project("joint", "协作空间", 300, joint = true, description = "一起交付"),
            project("archive", "聊天记忆", 50, systemKey = "chat_memory"),
            project("remote", "成员项目", 200, collaborationProjectId = "remote-1")
        )

        val all = groupProjectsForBrowser(projects, "")
        assertEquals(listOf(2, 0), all.personal.map(IndexedBrowserProject::index))
        assertEquals(listOf(1, 3), all.joint.map(IndexedBrowserProject::index))

        val byOwner = groupProjectsForBrowser(projects, "一龙")
        assertEquals(listOf(0), byOwner.personal.map(IndexedBrowserProject::index))
        assertTrue(byOwner.joint.isEmpty())

        val byDescription = groupProjectsForBrowser(projects, "交付")
        assertEquals(listOf(1), byDescription.joint.map(IndexedBrowserProject::index))
        assertTrue(byDescription.personal.isEmpty())
    }

    @Test
    fun eachGridRowKeepsFourEqualSlotsWithoutInventingAccessibleProjects() {
        val onlyProject = IndexedBrowserProject(
            index = 7,
            project = project("only", "唯一真实项目", 100)
        )

        val rows = projectBrowserGridRows(listOf(onlyProject))

        assertEquals(1, rows.size)
        assertEquals(PROJECT_BROWSER_COLUMN_COUNT, rows.single().size)
        assertEquals(onlyProject, rows.single().first())
        assertTrue(rows.single().drop(1).all { it == null })
        assertEquals(listOf(onlyProject), rows.flatten().filterNotNull())
    }

    @Test
    fun memberProjectsUseTheGroupSectionSemantic() {
        val groups = groupProjectsForBrowser(
            listOf(
                project("owner", "独立项目", 200, memberCount = 1),
                project("members", "成员项目", 100, memberCount = 3)
            ),
            ""
        )

        assertEquals(listOf("owner"), groups.personal.map { it.project.id })
        assertEquals(listOf("members"), groups.joint.map { it.project.id })
    }

    @Test
    fun projectBrowserTemporarilyOwnsTheOnlyBottomNavigationSelection() {
        val pageDestinations = listOf(
            MainBottomNavigationDestination.CHAT,
            MainBottomNavigationDestination.PROJECT,
            MainBottomNavigationDestination.PROFILE
        )

        pageDestinations.forEach { currentPage ->
            assertEquals(
                MainBottomNavigationDestination.MENU,
                selectedMainBottomNavigationDestination(currentPage, isProjectBrowserOpen = true)
            )
            assertEquals(
                currentPage,
                selectedMainBottomNavigationDestination(currentPage, isProjectBrowserOpen = false)
            )
        }
    }

    @Test
    fun androidAndWebUseTheSixOriginalPngAssets() {
        val expected = mapOf(
            "project_view_sheet_background.png" to "747bf7ed29d09582821ad7e36b8894f3a4cc8973ec9b641360ee55896d7d643c",
            "project_view_drag_handle.png" to "9a55996ca9ebc58685ffd9175f9e51047ab343e94f3bda3ec448c9a706696684",
            "project_view_avatar_placeholder.png" to "332e198031741c6679ea7c52e0013f79245ea34c9239cd27b291b55ebb169be3",
            "project_view_search_field.png" to "dbc787479c8672871896bde8cb9d3bd314adfc19c84804b10afa19a1efc3c769",
            "project_view_search_icon.png" to "8a0330578a5027032274ae72e600e413d750c85e761f41a8abf2271179a340b7",
            "project_view_chevron.png" to "3147797d74d7ee606612a79224216210865d6b783273fc3166eb332f3ed37275"
        )

        expected.forEach { (fileName, sha256) ->
            assertEquals(sha256, fileSha256(repositoryPath("android/app/src/main/res/drawable-nodpi/$fileName")))
            assertEquals(sha256, fileSha256(repositoryPath("server/src/assets/$fileName")))
        }
    }

    @Test
    fun androidAndWebKeepTheSameHighSheetContract() {
        val layout = readRepositoryFile("android/app/src/main/res/layout/activity_main.xml")
        val controller = readRepositoryFile("android/app/src/main/kotlin/com/elon/app/ProjectBrowserSheetController.kt")
        val web = readRepositoryFile("server/src/assets/web_page.html")

        assertTrue(layout.indexOf("@+id/projectBrowserSheet") < layout.indexOf("@+id/pageTabs"))
        assertTrue(layout.contains("android:layout_marginTop=\"10dp\""))
        assertTrue(layout.contains("@drawable/project_view_sheet_background"))
        assertTrue(controller.contains("projectBrowserGridRows(entries)"))
        assertTrue(controller.contains("LinearLayout.LayoutParams(0, dp(101), 1f)"))
        assertTrue(controller.contains("View.IMPORTANT_FOR_ACCESSIBILITY_NO"))
        assertTrue(controller.contains("TextUtils.TruncateAt.END"))
        assertTrue(controller.contains("dp(48)"))
        assertTrue(web.contains("top: max(10px, env(safe-area-inset-top));"))
        assertTrue(web.contains("env(safe-area-inset-bottom)"))
        assertTrue(web.contains("grid-template-columns: repeat(4"))
        assertTrue(web.contains("projectBrowserSearchInput.addEventListener('input', renderProjectBrowser)"))
        assertTrue(web.contains("selectProject(project)"))
    }

    @Test
    fun androidAndWebKeepProjectBrowserSelectionMutuallyExclusive() {
        val controller = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ProjectBrowserSheetController.kt"
        )
        val navigation = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainNavigationController.kt"
        )
        val web = readRepositoryFile("server/src/assets/web_page.html")

        assertTrue(controller.contains("onOpenChanged(true)"))
        assertTrue(controller.contains("onOpenChanged(false)"))
        val navigationState = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainBottomNavigationState.kt"
        )
        assertTrue(navigation.contains("MainBottomNavigationSelectionState("))
        assertTrue(navigationState.contains("selectedMainBottomNavigationDestination("))
        assertTrue(navigationState.contains("binding.bottomMenuSelection.isSelected = menuSelected"))
        assertTrue(web.contains("syncProjectBrowserNavigationSelection(true)"))
        assertTrue(web.contains("syncProjectBrowserNavigationSelection(false)"))
        assertTrue(web.contains("!isOpen && tab.dataset.tab === currentTab"))
    }

    private fun project(
        id: String,
        title: String,
        updatedAt: Long,
        owner: String? = null,
        joint: Boolean = false,
        collaborationProjectId: String? = null,
        description: String? = null,
        systemKey: String? = null,
        memberCount: Int? = null
    ) = AppProject(
        id = id,
        title = title,
        subtitle = "副标题",
        updatedAt = updatedAt,
        ownerAccount = owner,
        isJointProject = joint,
        collaborationProjectId = collaborationProjectId,
        projectDescription = description,
        systemProjectKey = systemKey,
        memberCount = memberCount
    )

    private fun readRepositoryFile(relativePath: String): String {
        return String(Files.readAllBytes(repositoryPath(relativePath)), StandardCharsets.UTF_8)
    }

    private fun repositoryPath(relativePath: String): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .map { it.resolve(relativePath) }
            .take(6)
            .firstOrNull(Files::isRegularFile)
            ?: error("Unable to find $relativePath from $cwd")
    }

    private fun fileSha256(path: Path): String {
        val digest = MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(path))
        return digest.joinToString("") { "%02x".format(it) }
    }
}
