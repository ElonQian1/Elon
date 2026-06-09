package com.elon.app

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.app.Dialog
import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.util.TypedValue
import android.view.Gravity
import android.view.KeyEvent
import android.view.View
import android.view.ViewGroup
import android.view.Window
import android.view.animation.AccelerateInterpolator
import android.view.animation.DecelerateInterpolator
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.EditText
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient

internal class MainProjectActions(
    private val activity: AppCompatActivity,
    private val projects: MutableList<AppProject>,
    private val activeProjectIndexProvider: () -> Int,
    private val setActiveProjectIndex: (Int) -> Unit,
    private val setActiveConversationIndex: (Int) -> Unit,
    private val titleEditText: (String) -> EditText,
    private val saveProjects: () -> Unit,
    private val renderProjectList: () -> Unit,
    private val openProject: (Int) -> Unit,
    private val openProjectSpace: (AppProject) -> Unit,
    private val showGitProjectDialog: () -> Unit,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val tokenProvider: () -> String?,
    private val isLoggedIn: () -> Boolean,
    private val removeSentProjectShareCards: (Set<String>) -> Int
) {
    private data class ProjectMenuAction(
        val title: String,
        val subtitle: String,
        val iconRes: Int,
        val action: () -> Unit
    )

    private data class ProjectDialogOrigin(
        val centerX: Float,
        val centerY: Float
    )

    private companion object {
        private const val PROJECT_DIALOG_ENTER_MS = 220L
        private const val PROJECT_DIALOG_EXIT_MS = 170L
        private const val PROJECT_DIALOG_START_SCALE = 0.22f
        const val MENU_ICON_COLOR = "#DDE8FC"
        const val MENU_ICON_BACKGROUND = "#283140"
    }

    fun showCreateProjectDialog() {
        val input = titleEditText("新项目 ${projects.size + 1}")
        val dialog = AlertDialog.Builder(activity)
            .setTitle("新建项目")
            .setView(input)
            .setNegativeButton("取消", null)
            .setPositiveButton("创建", null)
            .create()

        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val title = input.text.toString().trim()
                if (title.isBlank()) {
                    input.error = "请输入项目名称"
                    return@setOnClickListener
                }
                createProject(title)
                dialog.dismiss()
            }
        }
        dialog.show()
        input.selectAll()
    }

    fun showCreateJointProjectDialog() {
        if (!isLoggedIn()) {
            Toast.makeText(activity, "请先登录后发起联合项目", Toast.LENGTH_SHORT).show()
            return
        }
        val input = titleEditText("联合项目 ${projects.size + 1}")
        val dialog = AlertDialog.Builder(activity)
            .setTitle("发起联合项目")
            .setView(input)
            .setNegativeButton("取消", null)
            .setPositiveButton("创建", null)
            .create()

        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val title = input.text.toString().trim()
                if (title.isBlank()) {
                    input.error = "请输入项目名称"
                    return@setOnClickListener
                }
                createJointProject(title)
                dialog.dismiss()
            }
        }
        dialog.show()
        input.selectAll()
    }

    fun showProjectActions(index: Int, sourceView: View? = null) {
        if (index !in projects.indices) return
        val project = projects[index]
        val isJoint = project.isJointDevelopmentProject()

        val actions = buildProjectMenuActions(index, project, isJoint)
        val dialog = Dialog(activity, android.R.style.Theme_Translucent_NoTitleBar).apply {
            requestWindowFeature(Window.FEATURE_NO_TITLE)
        }
        val origin = projectDialogOriginFrom(sourceView)
        val scrim = View(activity).apply {
            setBackgroundColor(Color.BLACK)
            alpha = 0f
        }
        lateinit var panel: View
        var isClosing = false

        fun dismissWithAnimation(afterDismiss: () -> Unit = {}) {
            if (isClosing) return
            isClosing = true
            animateProjectActionsDialogOut(panel, scrim, origin) {
                dialog.setOnKeyListener(null)
                dialog.dismiss()
                afterDismiss()
            }
        }

        val content = createProjectActionsDialogView(project, isJoint, actions) { afterDismiss ->
            dismissWithAnimation(afterDismiss)
        }
        panel = content
        prepareProjectActionsDialogForEnter(panel)
        val root = FrameLayout(activity).apply {
            clipChildren = false
            clipToPadding = false
            addView(scrim, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            ))
            addView(panel, FrameLayout.LayoutParams(
                minOf(activity.resources.displayMetrics.widthPixels - dp(48), dp(360)),
                FrameLayout.LayoutParams.WRAP_CONTENT,
                Gravity.CENTER
            ))
        }
        scrim.setOnClickListener { dismissWithAnimation() }
        dialog.setContentView(root)
        dialog.setCanceledOnTouchOutside(false)
        dialog.setOnKeyListener { _, keyCode, event ->
            if (keyCode == KeyEvent.KEYCODE_BACK) {
                if (event.action == KeyEvent.ACTION_UP) {
                    dismissWithAnimation()
                }
                true
            } else {
                false
            }
        }
        dialog.setOnShowListener {
            val window = dialog.window ?: return@setOnShowListener
            window.setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            window.setDimAmount(0f)
            window.setWindowAnimations(0)
            window.attributes = window.attributes.apply {
                windowAnimations = 0
            }
            window.setLayout(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
            prepareProjectActionsDialogForEnter(panel)
            panel.post { animateProjectActionsDialogIn(panel, scrim, origin) }
        }
        dialog.show()
    }

    private fun projectDialogOriginFrom(sourceView: View?): ProjectDialogOrigin? {
        if (sourceView == null || sourceView.width <= 0 || sourceView.height <= 0) return null
        if (!sourceView.isAttachedToWindow) return null
        val location = IntArray(2)
        sourceView.getLocationOnScreen(location)
        return ProjectDialogOrigin(
            centerX = location[0] + sourceView.width / 2f,
            centerY = location[1] + sourceView.height / 2f
        )
    }

    private fun prepareProjectActionsDialogForEnter(panel: View) {
        panel.animate().setListener(null)
        panel.animate().cancel()
        panel.alpha = 0f
        panel.scaleX = PROJECT_DIALOG_START_SCALE
        panel.scaleY = PROJECT_DIALOG_START_SCALE
        panel.translationX = 0f
        panel.translationY = 0f
    }

    private fun animateProjectActionsDialogIn(panel: View, scrim: View, origin: ProjectDialogOrigin?) {
        val (startX, startY) = panelOffsetFromOrigin(panel, origin)
        panel.pivotX = panel.width / 2f
        panel.pivotY = panel.height / 2f
        panel.translationX = startX
        panel.translationY = startY
        scrim.animate()
            .alpha(0.62f)
            .setDuration(PROJECT_DIALOG_ENTER_MS)
            .setInterpolator(DecelerateInterpolator(1.2f))
            .start()
        panel.animate()
            .translationX(0f)
            .translationY(0f)
            .scaleX(1f)
            .scaleY(1f)
            .alpha(1f)
            .setDuration(PROJECT_DIALOG_ENTER_MS)
            .setInterpolator(DecelerateInterpolator(1.35f))
            .start()
    }

    private fun animateProjectActionsDialogOut(
        panel: View,
        scrim: View,
        origin: ProjectDialogOrigin?,
        afterEnd: () -> Unit
    ) {
        if (panel.width <= 0 || panel.height <= 0) {
            afterEnd()
            return
        }
        val (endX, endY) = panelOffsetFromOrigin(panel, origin)
        panel.animate().cancel()
        scrim.animate().cancel()
        panel.pivotX = panel.width / 2f
        panel.pivotY = panel.height / 2f
        scrim.animate()
            .alpha(0f)
            .setDuration(PROJECT_DIALOG_EXIT_MS)
            .setInterpolator(AccelerateInterpolator(1.05f))
            .start()
        panel.animate()
            .translationX(endX)
            .translationY(endY)
            .scaleX(PROJECT_DIALOG_START_SCALE)
            .scaleY(PROJECT_DIALOG_START_SCALE)
            .alpha(0f)
            .setDuration(PROJECT_DIALOG_EXIT_MS)
            .setInterpolator(AccelerateInterpolator(1.08f))
            .setListener(object : AnimatorListenerAdapter() {
                override fun onAnimationEnd(animation: Animator) {
                    panel.animate().setListener(null)
                    afterEnd()
                }
            })
            .start()
    }

    private fun panelOffsetFromOrigin(panel: View, origin: ProjectDialogOrigin?): Pair<Float, Float> {
        if (origin == null || panel.width <= 0 || panel.height <= 0) return 0f to 0f
        val location = IntArray(2)
        panel.getLocationOnScreen(location)
        val centerX = location[0] + panel.width / 2f
        val centerY = location[1] + panel.height / 2f
        return (origin.centerX - centerX) to (origin.centerY - centerY)
    }

    private fun buildProjectMenuActions(
        index: Int,
        project: AppProject,
        isJoint: Boolean
    ): List<ProjectMenuAction> {
        return buildList {
            add(
                ProjectMenuAction(
                    title = "编辑项目名称",
                    subtitle = "调整项目列表里的显示名称",
                    iconRes = R.drawable.ic_project_action_rename
                ) { showRenameProjectDialog(index) }
            )
            add(
                ProjectMenuAction(
                    title = "打开项目空间",
                    subtitle = if (isJoint) "进入联合协作空间" else "进入项目开发会话",
                    iconRes = R.drawable.ic_project_action_space
                ) {
                    setActiveProjectIndex(index)
                    saveProjects()
                    openProjectSpace(project)
                }
            )
            if (!isJoint) {
                add(
                    ProjectMenuAction(
                        title = "邀请好友协作",
                        subtitle = "生成项目卡片并开启协作",
                        iconRes = R.drawable.ic_project_action_invite
                    ) { confirmUpgradeToJoint(index) }
                )
            }
            if (isJoint) {
                add(
                    ProjectMenuAction(
                        title = "恢复为个人项目",
                        subtitle = "撤回协作状态与分享卡片",
                        iconRes = R.drawable.ic_project_action_restore
                    ) { confirmRestorePersonalProject(project) }
                )
                add(
                    ProjectMenuAction(
                        title = "发布到项目商城",
                        subtitle = "让其他用户可见并加入",
                        iconRes = R.drawable.ic_project_action_publish
                    ) { confirmPublishToMarketplace(project) }
                )
            }
            add(
                ProjectMenuAction(
                    title = "Git 仓库",
                    subtitle = "查看或配置项目远端",
                    iconRes = R.drawable.ic_project_action_git
                ) {
                    openProject(index)
                    showGitProjectDialog()
                }
            )
            add(
                ProjectMenuAction(
                    title = "协作权限 / 商城公开",
                    subtitle = "管理加入方式和可见范围",
                    iconRes = R.drawable.ic_project_action_visibility
                ) { showVisibilityDialog(project) }
            )
            if (projects.size > 1 && project.id != ELON_SELF_PROJECT_ID) {
                add(
                    ProjectMenuAction(
                        title = "删除项目",
                        subtitle = "从服务器和本机移除",
                        iconRes = R.drawable.ic_project_action_delete
                    ) { confirmDeleteProject(index) }
                )
            }
        }
    }

    private fun createProjectActionsDialogView(
        project: AppProject,
        isJoint: Boolean,
        actions: List<ProjectMenuAction>,
        dismissDialog: (() -> Unit) -> Unit
    ): View {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            background = GradientDrawable().apply {
                cornerRadius = dp(18).toFloat()
                setColor(Color.parseColor("#181B20"))
                setStroke(dp(1), Color.parseColor("#1E2126"))
            }
            setPadding(0, dp(18), 0, dp(8))

            addView(createProjectActionsHeader(project, isJoint))
            addView(createProjectActionsDivider(marginStart = dp(18), marginEnd = dp(18)))

            addView(ScrollView(activity).apply {
                isFillViewport = false
                overScrollMode = View.OVER_SCROLL_IF_CONTENT_SCROLLS
                addView(LinearLayout(activity).apply {
                    orientation = LinearLayout.VERTICAL
                    actions.forEachIndexed { actionIndex, action ->
                        addView(createProjectActionRow(action, dismissDialog))
                        if (actionIndex < actions.lastIndex) {
                            addView(createProjectActionsDivider(marginStart = dp(74), marginEnd = dp(18)))
                        }
                    }
                })
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ))
        }
    }

    private fun createProjectActionsHeader(project: AppProject, isJoint: Boolean): View {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(18), 0, dp(18), dp(16))

            addView(FrameLayout(activity).apply {
                background = GradientDrawable().apply {
                    shape = GradientDrawable.OVAL
                    setColor(Color.parseColor(MENU_ICON_BACKGROUND))
                }
                addView(ImageView(activity).apply {
                    setImageResource(R.drawable.ic_popup_project)
                    imageTintList = ColorStateList.valueOf(Color.parseColor(MENU_ICON_COLOR))
                }, FrameLayout.LayoutParams(dp(24), dp(24), Gravity.CENTER))
            }, LinearLayout.LayoutParams(dp(48), dp(48)))

            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = project.title.ifBlank { "未命名项目" }
                    setTextColor(Color.parseColor("#F2F5FA"))
                    setTypeface(typeface, Typeface.BOLD)
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, 20f)
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                })
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = projectStatusText(project, isJoint)
                    setTextColor(Color.parseColor(if (isJoint) "#58BE6A" else "#81B3D9"))
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, 12f)
                    background = GradientDrawable().apply {
                        cornerRadius = dp(9).toFloat()
                        setColor(Color.parseColor(if (isJoint) "#17351E" else "#152C3E"))
                    }
                    setPadding(dp(8), dp(4), dp(8), dp(4))
                }, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = dp(8)
                })
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(14)
            })
        }
    }

    private fun createProjectActionRow(
        action: ProjectMenuAction,
        dismissDialog: (() -> Unit) -> Unit
    ): View {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            minimumHeight = dp(62)
            setPadding(dp(18), dp(8), dp(18), dp(8))
            isClickable = true
            foreground = selectableForeground()

            addView(FrameLayout(activity).apply {
                background = GradientDrawable().apply {
                    shape = GradientDrawable.OVAL
                    setColor(Color.parseColor(MENU_ICON_BACKGROUND))
                }
                addView(ImageView(activity).apply {
                    setImageResource(action.iconRes)
                    imageTintList = ColorStateList.valueOf(Color.parseColor(MENU_ICON_COLOR))
                }, FrameLayout.LayoutParams(dp(22), dp(22), Gravity.CENTER))
            }, LinearLayout.LayoutParams(dp(40), dp(40)))

            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER_VERTICAL
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = action.title
                    setTextColor(Color.parseColor("#F2F5FA"))
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, 15.5f)
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                })
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = action.subtitle
                    setTextColor(Color.parseColor("#A6AFBD"))
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, 12.5f)
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                }, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = dp(5)
                })
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(16)
            })

            setOnClickListener {
                dismissDialog {
                    action.action()
                }
            }
        }
    }

    private fun createProjectActionsDivider(marginStart: Int = 0, marginEnd: Int = 0): View {
        return View(activity).apply {
            alpha = 0.75f
            setBackgroundColor(Color.parseColor("#1E2126"))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                1
            ).apply {
                this.marginStart = marginStart
                this.marginEnd = marginEnd
            }
        }
    }

    private fun projectStatusText(project: AppProject, isJoint: Boolean): String {
        if (!isJoint) return "个人项目"
        return when (normalizeProjectJoinMode(project.collaborationJoinMode)) {
            "open" -> "联合项目 · 商城公开"
            "readonly" -> "联合项目 · 广场只读"
            "approval" -> "联合项目 · 加入需审批"
            else -> "联合项目 · 邀请协作"
        }
    }

    private fun selectableForeground(): Drawable? = runCatching {
        val outValue = TypedValue()
        activity.theme.resolveAttribute(android.R.attr.selectableItemBackground, outValue, true)
        activity.getDrawable(outValue.resourceId)
    }.getOrNull()

    private fun dp(value: Int): Int {
        return (value * activity.resources.displayMetrics.density + 0.5f).toInt()
    }

    private fun confirmUpgradeToJoint(index: Int) {
        if (!isLoggedIn()) {
            Toast.makeText(activity, "请先登录再升级为联合项目", Toast.LENGTH_SHORT).show()
            return
        }
        val project = projects.getOrNull(index) ?: return
        AlertDialog.Builder(activity)
            .setTitle("邀请好友协作")
            .setMessage("「${project.title}」将同步至服务端并开启联合项目空间。\n\n这不会发布到项目商城；只有你把项目卡片发给好友或群聊后，收到卡片的人才能接受邀请共同开发。")
            .setPositiveButton("创建邀请") { _, _ -> doUpgradeToJoint(project) }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun doUpgradeToJoint(project: AppProject) {
        val token = tokenProvider() ?: run {
            Toast.makeText(activity, "登录已过期，请重新登录", Toast.LENGTH_SHORT).show()
            return
        }
        Toast.makeText(activity, "正在创建协作邀请...", Toast.LENGTH_SHORT).show()
        Thread {
            try {
                val created = createStoreProject(
                    http = http,
                    serverUrl = serverUrl,
                    name = project.title,
                    description = project.subtitle.takeIf { it.isNotBlank() },
                    token = token,
                    ownerAccount = AuthManager.displayName(activity)
                )
                setProjectVisibility(http, serverUrl, created.id, true, "invite", token)
                activity.runOnUiThread {
                    project.markJointDevelopment(created.id, "invite")
                    projects.indexOfFirst { it.id == project.id }
                        .takeIf { it >= 0 }
                        ?.let { setActiveProjectIndex(it) }
                    saveProjects()
                    renderProjectList()
                    openProjectSpace(project)
                    Toast.makeText(activity, "已创建邀请协作项目", Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(activity, "创建协作邀请失败：${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    private fun confirmPublishToMarketplace(project: AppProject) {
        AlertDialog.Builder(activity)
            .setTitle("发布到项目商城")
            .setMessage("「${project.title}」会出现在项目商城，所有用户都能看到并直接加入。\n\n如果只是邀请指定好友共同开发，请继续使用聊天里的项目卡片。")
            .setPositiveButton("发布商城") { _, _ -> doSetVisibility(project, true, "open") }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun confirmRestorePersonalProject(project: AppProject) {
        if (!project.isJointDevelopmentProject()) {
            Toast.makeText(activity, "已经是个人项目", Toast.LENGTH_SHORT).show()
            return
        }
        AlertDialog.Builder(activity)
            .setTitle("恢复为个人项目")
            .setMessage("「${project.title}」会从联合项目移回个人项目。\n\n如果它已经同步到服务端，会先设为私有，之后不会再作为联合项目空间打开。")
            .setPositiveButton("恢复") { _, _ -> doRestorePersonalProject(project) }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun doRestorePersonalProject(project: AppProject) {
        val collaborationId = project.collaborationProjectId?.trim().orEmpty()
        val hasRemoteProject = collaborationId.isNotBlank() || project.id.startsWith("prj_")
        val remoteProjectId = collaborationId.ifBlank { project.id }
        val cardProjectIds = setOf(project.id, remoteProjectId).map { it.trim() }.filter { it.isNotEmpty() }.toSet()
        if (!hasRemoteProject) {
            project.markPersonalDevelopment()
            saveProjects()
            renderProjectList()
            val localRemoved = removeSentProjectShareCards(cardProjectIds)
            Toast.makeText(activity, restorePersonalProjectToast(localRemoved), Toast.LENGTH_SHORT).show()
            return
        }
        if (!isLoggedIn()) {
            Toast.makeText(activity, "请先登录后恢复个人项目", Toast.LENGTH_SHORT).show()
            return
        }
        val token = tokenProvider() ?: run {
            Toast.makeText(activity, "登录已过期，请重新登录", Toast.LENGTH_SHORT).show()
            return
        }
        Toast.makeText(activity, "正在恢复个人项目...", Toast.LENGTH_SHORT).show()
        Thread {
            try {
                setProjectVisibility(http, serverUrl, remoteProjectId, false, "invite", token)
                val serverRemoved = cardProjectIds.sumOf { projectId ->
                    revokeProjectShareMessages(http, serverUrl, projectId, token)
                }
                activity.runOnUiThread {
                    project.markPersonalDevelopment()
                    saveProjects()
                    renderProjectList()
                    val localRemoved = removeSentProjectShareCards(cardProjectIds)
                    Toast.makeText(
                        activity,
                        restorePersonalProjectToast(maxOf(serverRemoved, localRemoved)),
                        Toast.LENGTH_SHORT
                    ).show()
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(activity, "恢复个人项目失败：${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    private fun restorePersonalProjectToast(removedCards: Int): String {
        return if (removedCards > 0) {
            "已恢复为个人项目，并撤回 $removedCards 张项目卡片"
        } else {
            "已恢复为个人项目"
        }
    }

    private fun showVisibilityDialog(project: AppProject) {
        if (!isLoggedIn()) {
            Toast.makeText(activity, "请先登录再修改项目可见性", Toast.LENGTH_SHORT).show()
            return
        }
        val options = arrayOf(
            "邀请协作（仅收到项目卡片的人可加入）",
            "发布到商城（所有人可见并可加入）",
            "广场只读体验（可进入、可问 AI、不能改代码）",
            "商城展示但需审批",
            "私有（仅成员可见）"
        )
        AlertDialog.Builder(activity)
            .setTitle("${project.title} · 协作权限")
            .setItems(options) { _, which ->
                val (isPublic, joinMode) = when (which) {
                    0 -> true to "invite"
                    1 -> true to "open"
                    2 -> true to "readonly"
                    3 -> true to "approval"
                    else -> false to "invite"
                }
                doSetVisibility(project, isPublic, joinMode)
            }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun doSetVisibility(project: AppProject, isPublic: Boolean, joinMode: String) {
        val token = tokenProvider() ?: run {
            Toast.makeText(activity, "未登录", Toast.LENGTH_SHORT).show()
            return
        }
        if (!isPublic && !project.isJointDevelopmentProject()) {
            project.markPersonalDevelopment()
            saveProjects()
            renderProjectList()
            Toast.makeText(activity, "已设为私有", Toast.LENGTH_SHORT).show()
            return
        }
        Thread {
            try {
                val remoteProjectId = if (isPublic && !project.isJointDevelopmentProject()) {
                    val created = createStoreProject(
                        http = http,
                        serverUrl = serverUrl,
                        name = project.title,
                        description = project.subtitle.takeIf { it.isNotBlank() },
                        token = token,
                        ownerAccount = AuthManager.displayName(activity)
                    )
                    setProjectVisibility(http, serverUrl, created.id, true, joinMode, token)
                    created.id
                } else {
                    val targetId = project.projectSpaceId()
                    setProjectVisibility(http, serverUrl, targetId, isPublic, joinMode, token)
                    targetId
                }
                val label = when {
                    !isPublic -> "已设为私有"
                    joinMode == "invite" -> "已设为邀请协作"
                    joinMode == "open" -> "已发布到项目商城"
                    joinMode == "readonly" -> "已发布到项目广场（只读体验）"
                    else -> "已发布到项目商城（需审批）"
                }
                activity.runOnUiThread {
                    if (isPublic) {
                        project.markJointDevelopment(remoteProjectId, joinMode)
                        saveProjects()
                        renderProjectList()
                    } else if (project.isJointDevelopmentProject()) {
                        project.markJointDevelopment(remoteProjectId, "invite")
                        saveProjects()
                        renderProjectList()
                    }
                    Toast.makeText(activity, label, Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(activity, "修改失败：${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    private fun createProject(title: String) {
        val token = tokenProvider() ?: run {
            Toast.makeText(activity, "请先登录并启动 PC 节点后新建项目", Toast.LENGTH_SHORT).show()
            return
        }
        Toast.makeText(activity, "正在通过 PC 节点创建项目...", Toast.LENGTH_SHORT).show()
        val ownerAccount = AuthManager.displayName(activity)
        Thread {
            try {
                val created = createStoreProject(
                    http = http,
                    serverUrl = serverUrl,
                    name = title,
                    description = "个人开发项目",
                    token = token,
                    ownerAccount = ownerAccount
                )
                activity.runOnUiThread {
                    val appProject = created.toOwnerAppProject()
                    val existingIndex = projects.indexOfFirst {
                        it.id == created.id || it.projectSpaceId() == created.id
                    }
                    val index = if (existingIndex >= 0) {
                        projects[existingIndex] = appProject
                        existingIndex
                    } else {
                        projects.add(appProject)
                        projects.lastIndex
                    }
                    setActiveProjectIndex(index)
                    setActiveConversationIndex(0)
                    saveProjects()
                    renderProjectList()
                    openProjectSpace(projects[index])
                    Toast.makeText(activity, "项目已在 PC 节点创建", Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(activity, "创建项目失败：${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    private fun createJointProject(title: String) {
        val token = tokenProvider() ?: run {
            Toast.makeText(activity, "登录已过期，请重新登录", Toast.LENGTH_SHORT).show()
            return
        }
        Toast.makeText(activity, "正在通过 PC 节点创建联合项目...", Toast.LENGTH_SHORT).show()
        val ownerAccount = AuthManager.displayName(activity)
        Thread {
            try {
                val created = createStoreProject(
                    http = http,
                    serverUrl = serverUrl,
                    name = title,
                    description = "联合开发项目",
                    token = token,
                    ownerAccount = ownerAccount
                )
                setProjectVisibility(http, serverUrl, created.id, true, "invite", token)
                activity.runOnUiThread {
                    val existingIndex = projects.indexOfFirst { it.projectSpaceId() == created.id }
                    val index = if (existingIndex >= 0) {
                        existingIndex
                    } else {
                        projects.add(created.toJointAppProject())
                        projects.lastIndex
                    }
                    projects[index].markJointDevelopment(created.id, "invite")
                    setActiveProjectIndex(index)
                    setActiveConversationIndex(0)
                    saveProjects()
                    renderProjectList()
                    openProjectSpace(projects[index])
                    Toast.makeText(activity, "联合项目已创建", Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(activity, "创建联合项目失败：${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    private fun showRenameProjectDialog(index: Int) {
        if (index !in projects.indices) return
        val project = projects[index]
        val input = titleEditText(project.title)
        val dialog = AlertDialog.Builder(activity)
            .setTitle("编辑项目名称")
            .setView(input)
            .setNegativeButton("取消", null)
            .setPositiveButton("保存", null)
            .create()

        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val title = input.text.toString().trim()
                if (title.isBlank()) {
                    input.error = "请输入项目名称"
                    return@setOnClickListener
                }
                project.title = summarize(title, 24)
                project.updatedAt = System.currentTimeMillis()
                saveProjects()
                renderProjectList()
                dialog.dismiss()
            }
        }
        dialog.show()
        input.selectAll()
    }

    private fun confirmDeleteProject(index: Int) {
        if (index !in projects.indices || projects.size <= 1) return
        val project = projects[index]
        if (project.id == ELON_SELF_PROJECT_ID) {
            Toast.makeText(activity, "一龙项目是平台自身项目，不能删除", Toast.LENGTH_SHORT).show()
            return
        }
        AlertDialog.Builder(activity)
            .setTitle("删除项目")
            .setMessage("会先从服务器删除「${project.title}」的项目记录、工作区文件、附件、构建产物和会话 worktree；成功后才从本机移除。正在运行的开发任务需先结束。")
            .setNegativeButton("取消", null)
            .setPositiveButton("删除") { _, _ -> deleteProject(index) }
            .show()
    }

    private fun deleteProject(index: Int) {
        if (index !in projects.indices || projects.size <= 1) return
        val project = projects[index]
        val targetProjectId = project.projectSpaceId()
        val token = tokenProvider()
        val userId = AuthManager.effectiveUserId(activity)
        Toast.makeText(activity, "正在删除服务器项目...", Toast.LENGTH_SHORT).show()
        Thread {
            try {
                val remoteDeleted = deleteServerProject(
                    http = http,
                    serverUrl = serverUrl,
                    projectId = targetProjectId,
                    token = token,
                    userId = userId
                )
                activity.runOnUiThread {
                    removeLocalProject(project.id, targetProjectId)
                    val message = if (remoteDeleted) {
                        "项目已从服务器和本机移除"
                    } else {
                        "服务器没有找到对应项目，已移除本机记录"
                    }
                    Toast.makeText(activity, message, Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(
                        activity,
                        "删除失败：${e.message}。本机未移除，避免服务器残留。",
                        Toast.LENGTH_LONG
                    ).show()
                }
            }
        }.start()
    }

    private fun removeLocalProject(localProjectId: String, remoteProjectId: String) {
        val deleteIndex = projects.indexOfFirst {
            it.id == localProjectId || it.projectSpaceId() == remoteProjectId
        }
        if (deleteIndex !in projects.indices || projects.size <= 1) return
        projects.removeAt(deleteIndex)
        val currentActive = activeProjectIndexProvider()
        val activeProjectIndex = when {
            currentActive > deleteIndex -> currentActive - 1
            currentActive >= projects.size -> projects.lastIndex
            else -> currentActive
        }.coerceIn(0, projects.lastIndex)
        setActiveProjectIndex(activeProjectIndex)
        val activeProject = projects[activeProjectIndex]
        if (activeProject.conversations.isEmpty()) {
            activeProject.conversations.add(defaultAppConversation())
        }
        setActiveConversationIndex(activeProject.activeConversationIndex.coerceIn(0, activeProject.conversations.lastIndex))
        saveProjects()
        renderProjectList()
    }
}
