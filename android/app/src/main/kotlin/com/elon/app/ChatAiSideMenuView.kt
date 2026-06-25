package com.elon.app

import android.animation.ValueAnimator
import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.animation.LinearInterpolator
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import kotlin.math.sin

internal class ChatAiSideMenuView(
    context: Context,
    private val conversations: () -> List<AppConversation>,
    private val activeConversationIndex: () -> Int,
    private val projects: () -> List<AppProject>,
    private val activeProjectIndex: () -> Int,
    private val openConversation: (Int) -> Unit,
    private val openPersonalProject: (Int) -> Unit,
    private val openJointProject: (Int) -> Unit,
    private val openProjectSpace: () -> Unit,
    private val copyConversationIdentity: (Int) -> Unit,
    private val isConversationWorking: (Int) -> Boolean,
    private val showCreateConversationDialog: () -> Unit,
    private val requestClose: (Boolean) -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) : FrameLayout(context) {
    private val menuContent = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(32), dp(92), dp(18), dp(18))
    }
    private val projectDirectoryGroup = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
    }
    private val chatSectionGap = View(context).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(CHAT_SECTION_GAP_COLLAPSED_DP)
        )
    }
    private val conversationDirectoryGroup = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
    }
    private val directoryRowAnimators = mutableMapOf<View, ValueAnimator>()
    private var personalProjectsExpanded = false
    private var jointProjectsExpanded = false

    init {
        clipChildren = false
        clipToPadding = false
        buildMenuContent()
    }

    fun render() {
        updateProjectSections()
        updateConversationSummaries()
    }

    fun stopAnimations() {
        directoryRowAnimators.values.forEach { it.cancel() }
        directoryRowAnimators.clear()
    }

    private fun buildMenuContent() {
        val menuScroll = ScrollView(context).apply {
            overScrollMode = View.OVER_SCROLL_NEVER
            isVerticalScrollBarEnabled = false
            isFillViewport = false
        }
        menuScroll.addView(
            menuContent,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
            )
        )
        addView(
            menuScroll,
            LayoutParams(
                LayoutParams.MATCH_PARENT,
                LayoutParams.MATCH_PARENT
            ).apply {
                gravity = Gravity.TOP or Gravity.START
                bottomMargin = dp(78)
            }
        )
        menuContent.addView(projectDirectoryGroup)
        menuContent.addView(chatSectionGap)
        menuContent.addView(conversationDirectoryGroup)
        conversationDirectoryGroup.addView(conversationHeaderRow())
    }

    private fun updateProjectSections() {
        projectDirectoryGroup.removeAllViews()
        addPersonalProjects()
        addJointProjects()
        addProjectSpaceEntry()
        updateChatSectionGap()
    }

    private fun addPersonalProjects() {
        projectDirectoryGroup.addView(sectionHeader("个人项目", personalProjectsExpanded) {
            personalProjectsExpanded = !personalProjectsExpanded
            updateProjectSections()
        })
        if (!personalProjectsExpanded) return

        val personalProjects = projects()
            .mapIndexed { index, project -> index to project }
            .filter { (_, project) -> !project.isJointDevelopmentProject() }
            .sortedWith(
                compareByDescending<Pair<Int, AppProject>> { it.second.isSystemArchiveProject() }
                    .thenByDescending { it.second.updatedAt }
            )
        if (personalProjects.isEmpty()) {
            projectDirectoryGroup.addView(emptyRow("暂无个人项目"))
        } else {
            personalProjects.forEach { (index, project) ->
                projectDirectoryGroup.addView(
                    projectNameRow(project, active = index == activeProjectIndex()) {
                        requestClose(true)
                        postDelayed({ openPersonalProject(index) }, PROJECT_OPEN_DELAY_MS)
                    }
                )
            }
        }
        projectDirectoryGroup.addView(space(34))
    }

    private fun addJointProjects() {
        projectDirectoryGroup.addView(sectionHeader("联合项目", jointProjectsExpanded) {
            jointProjectsExpanded = !jointProjectsExpanded
            updateProjectSections()
        })
        if (!jointProjectsExpanded) return

        val jointProjects = projects()
            .mapIndexed { index, project -> index to project }
            .filter { (_, project) -> project.isJointDevelopmentProject() }
            .sortedByDescending { it.second.updatedAt }
        if (jointProjects.isEmpty()) {
            projectDirectoryGroup.addView(emptyRow("暂无联合项目"))
            return
        }
        jointProjects.forEach { (index, project) ->
            projectDirectoryGroup.addView(
                projectNameRow(project, active = index == activeProjectIndex()) {
                    requestClose(true)
                    postDelayed({ openJointProject(index) }, PROJECT_OPEN_DELAY_MS)
                }
            )
        }
    }

    private fun addProjectSpaceEntry() {
        projectDirectoryGroup.addView(menuEntryRow("项目空间") {
            requestClose(true)
            postDelayed({ openProjectSpace() }, PROJECT_OPEN_DELAY_MS)
        })
    }

    private fun updateChatSectionGap() {
        val gapDp = if (!personalProjectsExpanded && !jointProjectsExpanded) {
            CHAT_SECTION_GAP_COLLAPSED_DP
        } else {
            CHAT_SECTION_GAP_EXPANDED_DP
        }
        val params = chatSectionGap.layoutParams as LinearLayout.LayoutParams
        params.height = dp(gapDp)
        chatSectionGap.layoutParams = params
    }

    private fun sectionHeader(
        title: String,
        expanded: Boolean,
        onClick: () -> Unit
    ): LinearLayout {
        return LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(46)
            ).apply {
                topMargin = dp(2)
            }
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            isClickable = true
            foreground = selectableForeground()
            contentDescription = if (expanded) "收起$title" else "展开$title"
            addView(
                menuText(title).apply {
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        LinearLayout.LayoutParams.MATCH_PARENT
                    )
                    setTextColor(Color.parseColor("#D6D6D6"))
                }
            )
            addView(sectionFolderIcon(expanded))
            setOnClickListener { onClick() }
        }
    }

    private fun sectionFolderIcon(expanded: Boolean): ImageView {
        return ImageView(context).apply {
            setImageResource(
                if (expanded) {
                    R.drawable.ic_side_menu_folder_open
                } else {
                    R.drawable.ic_side_menu_folder_closed
                }
            )
            imageTintList = ColorStateList.valueOf(Color.parseColor("#D6D6D6"))
            scaleType = ImageView.ScaleType.FIT_CENTER
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
            layoutParams = LinearLayout.LayoutParams(dp(28), dp(28)).apply {
                leftMargin = dp(16)
            }
        }
    }

    private fun menuEntryRow(title: String, onClick: () -> Unit): TextView {
        return menuText(title).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(46)
            ).apply {
                topMargin = dp(2)
            }
            setTextColor(Color.parseColor("#D6D6D6"))
            isClickable = true
            foreground = selectableForeground()
            contentDescription = title
            setOnClickListener { onClick() }
        }
    }

    private fun projectNameRow(project: AppProject, active: Boolean, onClick: () -> Unit): TextView {
        return menuText(project.title).apply {
            layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(34))
            setPadding(dp(10), 0, dp(10), 0)
            isClickable = true
            foreground = selectableForeground()
            setTextColor(Color.parseColor(if (active) "#D6D6D6" else "#A8A8A8"))
            if (active) {
                background = GradientDrawable().apply {
                    cornerRadius = dp(8).toFloat()
                    setColor(Color.parseColor("#222222"))
                }
            }
            setOnClickListener { onClick() }
        }
    }

    private fun updateConversationSummaries() {
        stopAnimations()
        while (conversationDirectoryGroup.childCount > 1) {
            conversationDirectoryGroup.removeViewAt(1)
        }
        val items = conversations()
        if (items.isEmpty()) {
            conversationDirectoryGroup.addView(directoryRow("暂无会话", active = false, working = false, onClick = {}))
            return
        }
        items.forEachIndexed { index, conversation ->
            conversationDirectoryGroup.addView(
                directoryRow(
                    title = conversation.title,
                    active = index == activeConversationIndex(),
                    working = isConversationWorking(index),
                    onClick = {
                        requestClose(true)
                        openConversation(index)
                    },
                    onLongClick = { copyConversationIdentity(index) }
                )
            )
        }
    }

    private fun conversationHeaderRow(): LinearLayout {
        return LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(44)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL

            addView(
                menuText("当前聊天").apply {
                    layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
                    setTextColor(Color.parseColor("#D6D6D6"))
                }
            )
            addView(
                ImageButton(context).apply {
                    setImageResource(R.drawable.ic_side_menu_new_chat)
                    imageTintList = ColorStateList.valueOf(Color.parseColor("#D6D6D6"))
                    background = null
                    scaleType = ImageView.ScaleType.FIT_CENTER
                    contentDescription = "新建会话"
                    isClickable = true
                    foreground = selectableForeground()
                    setPadding(dp(4), dp(4), dp(4), dp(4))
                    setOnClickListener {
                        requestClose(true)
                        postDelayed({ showCreateConversationDialog() }, DURATION_MS)
                    }
                },
                LinearLayout.LayoutParams(dp(38), dp(38)).apply {
                    rightMargin = dp(8)
                }
            )
        }
    }

    private fun directoryRow(
        title: String,
        active: Boolean,
        working: Boolean,
        onClick: () -> Unit,
        onLongClick: (() -> Unit)? = null
    ): TextView {
        return menuText(title).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(42)
            ).apply {
                topMargin = dp(4)
            }
            setPadding(dp(10), 0, dp(10), 0)
            isClickable = true
            foreground = selectableForeground()
            setTextColor(Color.parseColor(if (active) "#D6D6D6" else "#A8A8A8"))
            if (working) {
                startDirectoryRowShimmer(this)
            } else if (active) {
                background = GradientDrawable().apply {
                    cornerRadius = dp(8).toFloat()
                    setColor(Color.parseColor("#222222"))
                }
            }
            setOnClickListener { onClick() }
            setOnLongClickListener {
                onLongClick?.invoke()
                onLongClick != null
            }
        }
    }

    private fun menuText(title: String): TextView {
        return TextView(context).apply {
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            includeFontPadding = false
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            text = title
            setTextColor(Color.parseColor("#A8A8A8"))
            textSize = 17.5f
        }
    }

    private fun emptyRow(text: String): TextView {
        return menuText(text).apply {
            setTextColor(Color.parseColor("#777777"))
            textSize = 14f
            layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(34))
        }
    }

    private fun space(heightDp: Int): View {
        return View(context).apply {
            layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(heightDp))
        }
    }

    private fun startDirectoryRowShimmer(row: View) {
        val baseColor = Color.parseColor("#222222")
        val highlightColor = Color.parseColor("#2A2A2A")
        val background = GradientDrawable().apply {
            cornerRadius = dp(8).toFloat()
            setColor(baseColor)
        }
        row.background = background
        val animator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 1350L
            repeatCount = ValueAnimator.INFINITE
            repeatMode = ValueAnimator.RESTART
            interpolator = LinearInterpolator()
            addUpdateListener { valueAnimator ->
                val pulse = sin(Math.PI * valueAnimator.animatedFraction).toFloat()
                background.setColor(blendColor(baseColor, highlightColor, pulse))
            }
        }
        directoryRowAnimators[row] = animator
        row.addOnAttachStateChangeListener(object : View.OnAttachStateChangeListener {
            override fun onViewAttachedToWindow(v: View) = Unit
            override fun onViewDetachedFromWindow(v: View) {
                directoryRowAnimators.remove(v)?.cancel()
            }
        })
        animator.start()
    }

    private companion object {
        const val DURATION_MS = 260L
        const val CHAT_SECTION_GAP_COLLAPSED_DP = 148
        const val CHAT_SECTION_GAP_EXPANDED_DP = 28
        const val PROJECT_OPEN_DELAY_MS = 220L
    }
}
