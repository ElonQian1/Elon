package com.elon.app

import android.content.ClipData
import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.util.Base64
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject
import kotlin.concurrent.thread
import kotlin.math.roundToInt

internal class ChatProjectSideMenuView(
    context: Context,
    private val projects: () -> List<AppProject>,
    private val activeProjectIndex: () -> Int,
    private val openPersonalProject: (Int) -> Unit,
    private val openJointProject: (Int) -> Unit,
    private val openProjectCenter: () -> Unit,
    private val requestClose: (Boolean) -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) : FrameLayout(context) {
    private val contentScroll = ScrollView(context).apply {
        overScrollMode = OVER_SCROLL_NEVER
        isVerticalScrollBarEnabled = false
        isFillViewport = false
    }
    private val content = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(32), dp(58), dp(26), dp(22))
    }
    private val profileDock = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(32), dp(12), dp(26), dp(18))
        background = GradientDrawable().apply {
            setColor(Color.parseColor("#0D0D0D"))
        }
    }
    private val levelMetaLeft = dockMetaText("Lv.--")
    private val levelMetaRight = dockMetaText("--")
    private val levelProgress = LevelProgressBarView(context).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(10)
        ).apply {
            topMargin = dp(8)
        }
    }
    private val profileNameText = TextView(context).apply {
        includeFontPadding = false
        maxLines = 1
        ellipsize = TextUtils.TruncateAt.END
        setTextColor(Color.parseColor("#D9D9D9"))
        textSize = 22f
    }
    private val profileStatusText = TextView(context).apply {
        includeFontPadding = false
        maxLines = 1
        ellipsize = TextUtils.TruncateAt.END
        setTextColor(Color.parseColor("#777777"))
        textSize = 14f
        setPadding(0, dp(7), 0, 0)
    }
    private var avatarView: View? = null
    private var personalProjectsExpanded = false
    private var jointProjectsExpanded = false
    private var progressionState: ProgressionState? = null
    private var progressionLoading = false
    private var lastProgressionLoadAt = 0L

    init {
        clipChildren = false
        clipToPadding = false
        contentScroll.addView(
            content,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
            )
        )
        addView(
            contentScroll,
            LayoutParams(
                LayoutParams.MATCH_PARENT,
                LayoutParams.MATCH_PARENT
            ).apply {
                bottomMargin = dp(PROFILE_DOCK_HEIGHT_DP)
            }
        )
        addView(
            profileDock,
            LayoutParams(
                LayoutParams.MATCH_PARENT,
                dp(PROFILE_DOCK_HEIGHT_DP)
            ).apply {
                gravity = Gravity.BOTTOM or Gravity.START
            }
        )
        buildProfileDock()
    }

    fun render() {
        content.removeAllViews()
        addTitle()
        addSearchPill()
        addProjectCenterSection()
        addPersonalProjects()
        addJointProjects()
        updateProfileDock()
        refreshProgressionIfNeeded()
    }

    private fun addTitle() {
        content.addView(TextView(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(48)
            )
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = "项目"
            setTextColor(Color.parseColor("#D9D9D9"))
            textSize = 22f
            setTypeface(typeface, Typeface.NORMAL)
        })
    }

    private fun addSearchPill() {
        val box = FrameLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(56)
            ).apply {
                topMargin = dp(30)
            }
            background = roundedRect("#454545", dp(28))
            isClickable = true
            foreground = selectableForeground()
            contentDescription = "搜索项目"
            setOnClickListener {
                requestClose(true)
                postDelayed({ openProjectCenter() }, CLOSE_DELAY_MS)
            }
        }
        box.addView(
            ImageView(context).apply {
                setImageResource(R.drawable.ic_top_search_custom)
                imageTintList = ColorStateList.valueOf(Color.parseColor("#D9D9D9"))
                scaleType = ImageView.ScaleType.FIT_CENTER
                importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
            },
            LayoutParams(dp(32), dp(32)).apply {
                gravity = Gravity.START or Gravity.CENTER_VERTICAL
                leftMargin = dp(22)
            }
        )
        content.addView(box)
    }

    private fun addProjectCenterSection() {
        content.addView(projectCenterRow())
        content.addView(recommendHeader())
        content.addView(featuredProjectBlock())
    }

    private fun projectCenterRow(): LinearLayout {
        return LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(48)
            ).apply {
                topMargin = dp(44)
            }
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            isClickable = true
            foreground = selectableForeground()
            contentDescription = "打开项目中心"
            setOnClickListener {
                requestClose(true)
                postDelayed({ openProjectCenter() }, CLOSE_DELAY_MS)
            }
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
                gravity = Gravity.CENTER_VERTICAL or Gravity.START
                includeFontPadding = false
                text = "项目中心"
                setTextColor(Color.parseColor("#D9D9D9"))
                textSize = 22f
            })
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(dp(42), LinearLayout.LayoutParams.MATCH_PARENT)
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = "›"
                setTextColor(Color.parseColor("#D9D9D9"))
                textSize = 36f
            })
        }
    }

    private fun recommendHeader(): LinearLayout {
        return LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(42)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            addView(TextView(context).apply {
                includeFontPadding = false
                text = "推荐"
                setTextColor(Color.parseColor("#D9D9D9"))
                textSize = 22f
            })
            addView(TextView(context).apply {
                includeFontPadding = false
                text = " ↪"
                setTextColor(Color.parseColor("#D9D9D9"))
                textSize = 24f
                setPadding(dp(8), 0, 0, 0)
            })
        }
    }

    private fun featuredProjectBlock(): LinearLayout {
        val entry = featuredProjectEntry()
        val project = entry?.second
        return LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            orientation = LinearLayout.VERTICAL
            addView(featuredProjectSummary(entry))
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = dp(20)
                }
                includeFontPadding = false
                maxLines = 2
                ellipsize = TextUtils.TruncateAt.END
                text = "应用介绍：${project?.projectCardIntroduction() ?: "一款自动化电商工作流的AI智能"}"
                setTextColor(Color.parseColor("#D9D9D9"))
                textSize = 16f
                setLineSpacing(dp(3).toFloat(), 1f)
            })
            addView(previewStrip())
            addView(View(context).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    1
                ).apply {
                    topMargin = dp(30)
                    leftMargin = dp(48)
                    rightMargin = dp(48)
                }
                setBackgroundColor(Color.parseColor("#222222"))
            })
        }
    }

    private fun featuredProjectSummary(entry: Pair<Int, AppProject>?): LinearLayout {
        val project = entry?.second
        return LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(76)
            ).apply {
                topMargin = dp(10)
            }
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            isClickable = project != null
            foreground = if (project != null) selectableForeground() else null
            contentDescription = project?.title?.let { "打开项目 $it" } ?: "暂无推荐项目"
            setOnClickListener {
                entry?.let { openProjectEntry(it.first, it.second) }
            }
            addView(projectCover(project))
            addView(LinearLayout(context).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                    leftMargin = dp(16)
                }
                orientation = LinearLayout.VERTICAL
                addView(TextView(context).apply {
                    includeFontPadding = false
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                    text = project?.title?.takeIf { it.isNotBlank() } ?: "项目名称"
                    setTextColor(Color.parseColor("#D9D9D9"))
                    textSize = 21f
                })
                addView(metaText("创建者：${projectCreatorLabel(project)}"))
                addView(metaText("版本：1.0"))
            })
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(dp(48), dp(56))
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = "↗"
                setTextColor(Color.parseColor("#D9D9D9"))
                textSize = 34f
            })
        }
    }

    private fun previewStrip(): LinearLayout {
        return LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(148)
            ).apply {
                topMargin = dp(28)
            }
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            addView(previewCell())
            addView(previewCell().apply {
                (layoutParams as LinearLayout.LayoutParams).leftMargin = dp(12)
            })
        }
    }

    private fun previewCell(): FrameLayout {
        return FrameLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
            background = GradientDrawable().apply {
                cornerRadius = dp(8).toFloat()
                setColor(Color.TRANSPARENT)
                setStroke(dp(1), Color.parseColor("#5A5A5A"))
            }
            addView(
                ImageView(context).apply {
                    setImageResource(R.drawable.ic_project_preview_placeholder)
                    imageTintList = ColorStateList.valueOf(Color.parseColor("#D9D9D9"))
                    scaleType = ImageView.ScaleType.FIT_CENTER
                    alpha = 0.9f
                    importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                },
                LayoutParams(dp(34), dp(34)).apply {
                    gravity = Gravity.CENTER
                }
            )
        }
    }

    private fun addPersonalProjects() {
        content.addView(space(46))
        content.addView(sectionHeader("你的创建", personalProjectsExpanded) {
            personalProjectsExpanded = !personalProjectsExpanded
            render()
        })
        if (!personalProjectsExpanded) return

        val list = personalProjectEntries()
        if (list.isEmpty()) {
            content.addView(emptyRow("暂无你的创建"))
            return
        }
        list.forEach { (index, project) ->
            content.addView(projectNameRow(project, active = index == activeProjectIndex()) {
                openProjectEntry(index, project)
            })
        }
    }

    private fun addJointProjects() {
        content.addView(sectionHeader("联合开发", jointProjectsExpanded) {
            jointProjectsExpanded = !jointProjectsExpanded
            render()
        })
        if (!jointProjectsExpanded) return

        val jointProjects = jointProjectEntries()
        if (jointProjects.isEmpty()) {
            content.addView(emptyRow("暂无联合开发"))
            return
        }
        jointProjects.forEach { (index, project) ->
            content.addView(projectNameRow(project, active = index == activeProjectIndex()) {
                openProjectEntry(index, project)
            })
        }
    }

    private fun sectionHeader(
        title: String,
        expanded: Boolean,
        onClick: () -> Unit
    ): LinearLayout {
        return LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(54)
            ).apply {
                topMargin = dp(8)
            }
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            isClickable = true
            foreground = selectableForeground()
            contentDescription = if (expanded) "收起$title" else "展开$title"
            addView(menuText(title).apply {
                setTextColor(Color.parseColor("#D6D6D6"))
                textSize = 22f
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.MATCH_PARENT
                )
            })
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
            layoutParams = LinearLayout.LayoutParams(dp(32), dp(32)).apply {
                leftMargin = dp(18)
            }
        }
    }

    private fun projectNameRow(project: AppProject, active: Boolean, onClick: () -> Unit): TextView {
        return menuText(project.title).apply {
            layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(34))
            setPadding(dp(10), 0, dp(10), 0)
            isClickable = true
            foreground = selectableForeground()
            if (active) {
                background = GradientDrawable().apply {
                    cornerRadius = dp(8).toFloat()
                    setColor(Color.parseColor("#222222"))
                }
            }
            setOnClickListener { onClick() }
            setOnLongClickListener {
                startProjectDrag(it, project.toChatProjectShare())
                true
            }
        }
    }

    private fun startProjectDrag(source: View, share: ChatProjectShare) {
        val clip = ClipData.newPlainText("project", share.toMessageText())
        source.startDragAndDrop(clip, View.DragShadowBuilder(source), share, 0)
    }

    private fun emptyRow(text: String): TextView {
        return menuText(text).apply {
            setTextColor(Color.parseColor("#777777"))
            textSize = 14f
            layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(34))
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
            textSize = 17f
        }
    }

    private fun metaText(value: String): TextView {
        return TextView(context).apply {
            includeFontPadding = false
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            text = value
            setTextColor(Color.parseColor("#777777"))
            textSize = 15f
            setPadding(0, dp(6), 0, 0)
        }
    }

    private fun space(heightDp: Int): View {
        return View(context).apply {
            layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(heightDp))
        }
    }

    private fun personalProjectEntries(): List<Pair<Int, AppProject>> =
        projects()
            .mapIndexed { index, project -> index to project }
            .filter { (_, project) -> !project.isJointDevelopmentProject() }
            .sortedWith(
                compareByDescending<Pair<Int, AppProject>> { it.second.isSystemArchiveProject() }
                    .thenByDescending { it.second.updatedAt }
            )

    private fun jointProjectEntries(): List<Pair<Int, AppProject>> =
        projects()
            .mapIndexed { index, project -> index to project }
            .filter { (_, project) -> project.isJointDevelopmentProject() }
            .sortedByDescending { it.second.updatedAt }

    private fun featuredProjectEntry(): Pair<Int, AppProject>? {
        val projectList = projects()
        val active = activeProjectIndex().takeIf { it in projectList.indices }?.let { it to projectList[it] }
        val indexed = projectList.mapIndexed { index, project -> index to project }
        return active
            ?: indexed.firstOrNull { (_, project) -> project.isJointDevelopmentProject() }
            ?: indexed.maxByOrNull { it.second.updatedAt }
    }

    private fun openProjectEntry(index: Int, project: AppProject) {
        requestClose(true)
        postDelayed({
            if (project.isJointDevelopmentProject()) {
                openJointProject(index)
            } else {
                openPersonalProject(index)
            }
        }, CLOSE_DELAY_MS)
    }

    private fun projectCover(project: AppProject?): FrameLayout {
        val cover = FrameLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(dp(64), dp(64))
            background = roundedRect("#D9D9D9", dp(10))
            clipToOutline = true
        }
        decodeDataUrlBitmap(project?.iconDataUrl)?.let { bitmap ->
            cover.addView(
                ImageView(context).apply {
                    setImageBitmap(bitmap)
                    scaleType = ImageView.ScaleType.CENTER_CROP
                    contentDescription = null
                },
                LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT)
            )
        }
        return cover
    }

    private fun projectCreatorLabel(project: AppProject?): String {
        if (project == null) return "叶云"
        project.ownerAccount?.trim()?.takeIf { it.isNotBlank() }?.let { return it }
        return project.projectOriginLabel().removeSuffix("创建").ifBlank { "叶云" }
    }

    private fun roundedRect(color: String, radius: Int): GradientDrawable =
        GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            cornerRadius = radius.toFloat()
            setColor(Color.parseColor(color))
        }

    private fun dockMetaText(value: String): TextView {
        return TextView(context).apply {
            includeFontPadding = false
            text = value
            setTextColor(Color.parseColor("#D9D9D9"))
            textSize = 14f
        }
    }

    private fun buildProfileDock() {
        profileDock.addView(LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            addView(levelMetaLeft, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            addView(levelMetaRight)
        })
        profileDock.addView(levelProgress)
        profileDock.addView(LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(66)
            ).apply {
                topMargin = dp(14)
            }
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            addView(TextView(context).also { avatarView = it }, LinearLayout.LayoutParams(dp(56), dp(56)))
            addView(LinearLayout(context).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                    leftMargin = dp(18)
                }
                orientation = LinearLayout.VERTICAL
                addView(profileNameText)
                addView(profileStatusText)
            })
        })
    }

    private fun updateProfileDock() {
        val profile = UserProfileStore.load(context)
        profileNameText.text = if (AuthManager.isLoggedIn(context)) profile.displayName else "未登录"
        profileStatusText.text = if (AuthManager.isLoggedIn(context)) "在线" else "需要登录"
        replaceAvatar(createDockAvatar(profile))
        applyProgressionState()
    }

    private fun replaceAvatar(nextAvatar: View) {
        val currentAvatar = avatarView ?: return
        val parent = currentAvatar.parent as? ViewGroup ?: return
        val index = parent.indexOfChild(currentAvatar)
        if (index < 0) return
        parent.removeViewAt(index)
        parent.addView(nextAvatar, index, LinearLayout.LayoutParams(dp(56), dp(56)))
        avatarView = nextAvatar
    }

    private fun createDockAvatar(profile: UserProfile): View {
        val bitmap = UserProfileStore.decodeAvatar(profile.avatarDataUrl)
        if (bitmap != null) {
            return ImageView(context).apply {
                background = GradientDrawable().apply {
                    shape = GradientDrawable.OVAL
                    setColor(Color.parseColor("#D9D9D9"))
                }
                clipToOutline = true
                scaleType = ImageView.ScaleType.CENTER_CROP
                setImageBitmap(bitmap)
                contentDescription = "头像"
            }
        }
        return TextView(context).apply {
            background = GradientDrawable().apply {
                shape = GradientDrawable.OVAL
                setColor(Color.parseColor("#D9D9D9"))
            }
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = UserProfileStore.avatarInitial(profile.displayName)
            setTextColor(Color.parseColor("#101010"))
            textSize = 22f
            setTypeface(typeface, Typeface.BOLD)
            contentDescription = "头像"
        }
    }

    private fun refreshProgressionIfNeeded() {
        if (!AuthManager.isLoggedIn(context)) {
            progressionState = null
            progressionLoading = false
            applyProgressionState()
            return
        }
        val now = System.currentTimeMillis()
        if (progressionLoading || now - lastProgressionLoadAt < PROGRESSION_REFRESH_INTERVAL_MS) return
        progressionLoading = true
        lastProgressionLoadAt = now
        val appContext = context.applicationContext
        thread(name = "project-side-menu-progression") {
            val result = runCatching { fetchProgression(appContext) }.getOrNull()
            post {
                progressionLoading = false
                result?.let { progressionState = it }
                applyProgressionState()
            }
        }
    }

    private fun applyProgressionState() {
        val state = progressionState
        if (state == null) {
            levelMetaLeft.text = if (AuthManager.isLoggedIn(context)) "Lv.--" else "Lv.0"
            levelMetaRight.text = if (AuthManager.isLoggedIn(context)) "同步中" else "0%"
            levelProgress.setSegments(floatArrayOf(0f, 0f, 0f, 0f))
            return
        }
        levelMetaLeft.text = "Lv.${state.level}"
        levelMetaRight.text = "${state.percent}%"
        levelProgress.setSegments(state.segments)
    }

    private fun fetchProgression(appContext: Context): ProgressionState {
        val token = AuthManager.token(appContext) ?: error("未登录")
        val url = BuildConfig.SERVER_URL.trimEnd('/') + "/api/me/progression"
        val request = Request.Builder()
            .url(url)
            .header("Authorization", "Bearer $token")
            .get()
            .build()
        progressionHttp.newCall(request).execute().use { response ->
            val body = response.body?.string().orEmpty()
            if (!response.isSuccessful) error("等级同步失败")
            return parseProgression(JSONObject(body))
        }
    }

    private fun parseProgression(json: JSONObject): ProgressionState {
        val level = json.optInt("level", 1).coerceAtLeast(1)
        val percent = (json.optDouble("level_progress_ratio", 0.0).coerceIn(0.0, 1.0) * 100)
            .roundToInt()
            .coerceIn(0, 100)
        val segments = if (json.has("own_codex_progress_ratio")) {
            floatArrayOf(
                json.optRatio("own_codex_progress_ratio"),
                json.optRatio("platform_progress_ratio"),
                json.optRatio("shared_codex_progress_ratio"),
                json.optRatio("provided_progress_ratio")
            )
        } else {
            floatArrayOf(
                json.optRatio("consumed_progress_ratio"),
                0f,
                0f,
                json.optRatio("provided_progress_ratio")
            )
        }
        return ProgressionState(level, percent, segments)
    }

    private fun JSONObject.optRatio(key: String): Float =
        optDouble(key, 0.0).coerceIn(0.0, 1.0).toFloat()

    private fun decodeDataUrlBitmap(dataUrl: String?): Bitmap? {
        val data = dataUrl?.substringAfter(",", "")?.takeIf { it.isNotBlank() } ?: return null
        return runCatching {
            val bytes = Base64.decode(data, Base64.DEFAULT)
            BitmapFactory.decodeByteArray(bytes, 0, bytes.size)
        }.getOrNull()
    }

    private companion object {
        const val CLOSE_DELAY_MS = 220L
        const val PROFILE_DOCK_HEIGHT_DP = 148
        const val PROGRESSION_REFRESH_INTERVAL_MS = 60_000L
        val progressionHttp = OkHttpClient()
    }
}
