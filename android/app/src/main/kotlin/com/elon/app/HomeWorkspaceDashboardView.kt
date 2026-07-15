package com.elon.app

import android.graphics.Typeface
import android.text.TextUtils
import android.util.TypedValue
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.widget.FrameLayout
import android.widget.HorizontalScrollView
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.graphics.drawable.RoundedBitmapDrawableFactory

internal data class HomeWorkspaceSurface(
    val friendsPanel: FrameLayout,
    val friendRows: LinearLayout
) {
    fun contains(event: MotionEvent): Boolean {
        if (!friendsPanel.isShown) return false
        val location = IntArray(2)
        friendsPanel.getLocationOnScreen(location)
        return event.rawX >= location[0] &&
            event.rawX <= location[0] + friendsPanel.width &&
            event.rawY >= location[1] &&
            event.rawY <= location[1] + friendsPanel.height
    }
}

internal class HomeWorkspaceDashboardView(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> android.graphics.drawable.Drawable?,
    private val showCreateProjectDialog: () -> Unit,
    private val showProjectHome: () -> Unit,
    private val showAddFriendDialog: () -> Unit,
    private val openProject: (Int) -> Unit,
    private val showProjectActions: (Int, View?) -> Unit
) {
    fun render(
        root: LinearLayout,
        projects: List<AppProject>,
        friendSectionTitle: String
    ): HomeWorkspaceSurface {
        root.addView(createProjectSection(projects))
        val surface = createFriendsPanel(friendSectionTitle)
        root.addView(
            surface.friendsPanel,
            LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        )
        updateFriendsPanelMinimumHeight(root, surface.friendsPanel)
        return surface
    }

    private fun createProjectSection(projects: List<AppProject>): LinearLayout {
        return LinearLayout(activity).apply {
            id = R.id.homeWorkspaceProjectSection
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(PROJECT_SECTION_HEIGHT_DP)
            )
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(activity.getColor(R.color.elon_bg_app))
            addView(createProjectHeader())
            addView(createProjectStrip(projects))
        }
    }

    private fun createProjectHeader(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(PROJECT_HEADER_HEIGHT_DP)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(20), 0, dp(8), 0)

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "项目"
                setTextColor(activity.getColor(R.color.elon_text_primary))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 16f)
                translationY = -dp(6).toFloat()
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))

            addView(FrameLayout(activity).apply {
                id = R.id.homeWorkspaceProjectsButton
                contentDescription = "查看全部项目"
                isClickable = true
                isFocusable = true
                foreground = selectableForeground()
                setOnClickListener { showProjectHome() }
                addView(ImageView(activity).apply {
                    importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                    scaleType = ImageView.ScaleType.FIT_CENTER
                    setImageResource(R.drawable.ic_home_workspace_chevron)
                    translationY = -dp(6).toFloat()
                }, FrameLayout.LayoutParams(dp(16), dp(16), Gravity.CENTER))
            }, LinearLayout.LayoutParams(dp(48), dp(44)))
        }
    }

    private fun createProjectStrip(projects: List<AppProject>): HorizontalScrollView {
        return HorizontalScrollView(activity).apply {
            id = R.id.homeWorkspaceProjectStrip
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                0,
                1f
            )
            isHorizontalScrollBarEnabled = false
            overScrollMode = View.OVER_SCROLL_NEVER
            clipToPadding = false
            addView(LinearLayout(activity).apply {
                gravity = Gravity.TOP
                orientation = LinearLayout.HORIZONTAL
                setPadding(dp(21), dp(3), dp(12), 0)
                addProjectStripItem(
                    createProjectAddItem(),
                    gapAfterDp = if (projects.isNotEmpty()) PROJECT_ADD_GAP_DP else 0
                )
                projects.forEachIndexed { index, project ->
                    addProjectStripItem(
                        createProjectItem(index, project),
                        gapAfterDp = 0
                    )
                }
            })
        }
    }

    private fun LinearLayout.addProjectStripItem(item: View, gapAfterDp: Int) {
        addView(item, LinearLayout.LayoutParams(dp(PROJECT_ITEM_WIDTH_DP), dp(PROJECT_ITEM_HEIGHT_DP)).apply {
            marginEnd = dp(gapAfterDp)
        })
    }

    private fun createProjectAddItem(): View {
        return createProjectTile(label = "新增", contentDescription = "新增项目") { tile ->
            tile.addView(ImageView(activity).apply {
                importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                scaleType = ImageView.ScaleType.FIT_XY
                setImageResource(R.drawable.bg_home_workspace_project_add_outline)
            }, FrameLayout.LayoutParams(dp(56), dp(56), Gravity.TOP or Gravity.CENTER_HORIZONTAL))
            tile.addView(ImageView(activity).apply {
                importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                scaleType = ImageView.ScaleType.FIT_XY
                setImageResource(R.drawable.bg_home_workspace_project_add_inner)
            }, FrameLayout.LayoutParams(dp(46), dp(46), Gravity.TOP or Gravity.CENTER_HORIZONTAL).apply {
                topMargin = dp(5)
            })
            tile.addView(ImageView(activity).apply {
                importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                scaleType = ImageView.ScaleType.FIT_CENTER
                setImageResource(R.drawable.ic_side_menu_new_chat)
            }, FrameLayout.LayoutParams(dp(26), dp(26), Gravity.TOP or Gravity.CENTER_HORIZONTAL).apply {
                topMargin = dp(15)
            })
        }.apply {
            setOnClickListener { showCreateProjectDialog() }
        }
    }

    private fun createProjectItem(index: Int, project: AppProject): View {
        val label = project.title.ifBlank { "项目名" }
        return createProjectTile(label = label, contentDescription = "打开项目 $label") { tile ->
            val image = ImageView(activity).apply {
                importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                scaleType = ImageView.ScaleType.CENTER_CROP
                val bitmap = UserProfileStore.decodeAvatar(project.iconDataUrl)
                if (bitmap == null) {
                    setImageResource(R.drawable.bg_home_workspace_project_placeholder)
                } else {
                    setImageDrawable(RoundedBitmapDrawableFactory.create(resources, bitmap).apply {
                        isCircular = true
                    })
                }
            }
            tile.addView(image, FrameLayout.LayoutParams(dp(56), dp(56), Gravity.TOP or Gravity.CENTER_HORIZONTAL))
        }.apply {
            setOnClickListener { openProject(index) }
            setOnLongClickListener { anchor ->
                showProjectActions(index, anchor)
                true
            }
        }
    }

    private fun createProjectTile(
        label: String,
        contentDescription: String,
        decorateCircle: (FrameLayout) -> Unit
    ): FrameLayout {
        return FrameLayout(activity).apply {
            this.contentDescription = contentDescription
            isClickable = true
            isFocusable = true
            foreground = selectableForeground()
            decorateCircle(this)
            addView(TextView(activity).apply {
                includeFontPadding = false
                ellipsize = TextUtils.TruncateAt.END
                gravity = Gravity.CENTER
                maxLines = 1
                text = label
                setTextColor(activity.getColor(R.color.elon_text_primary))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 16f)
                setTypeface(typeface, Typeface.NORMAL)
            }, FrameLayout.LayoutParams(dp(PROJECT_ITEM_WIDTH_DP), dp(28), Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL))
        }
    }

    private fun createFriendsPanel(title: String): HomeWorkspaceSurface {
        val panel = FrameLayout(activity).apply {
            id = R.id.homeWorkspaceFriendsPanel
            contentDescription = "好友面板"
            clipChildren = false
            clipToPadding = false
            addView(ImageView(activity).apply {
                importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                scaleType = ImageView.ScaleType.FIT_XY
                setImageResource(R.drawable.bg_home_workspace_friends_panel)
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            ))
            addView(ImageView(activity).apply {
                importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                scaleType = ImageView.ScaleType.FIT_XY
                setImageResource(R.drawable.bg_home_workspace_drag_handle)
            }, FrameLayout.LayoutParams(dp(66), dp(5), Gravity.TOP or Gravity.CENTER_HORIZONTAL).apply {
                topMargin = dp(12)
            })
        }
        val content = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, dp(21), 0, dp(88))
        }
        val rows = LinearLayout(activity).apply {
            id = R.id.homeWorkspaceFriendRows
            orientation = LinearLayout.VERTICAL
            clipChildren = false
            clipToPadding = false
        }
        content.addView(createFriendsHeader(title))
        content.addView(rows, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ))
        panel.addView(content, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.WRAP_CONTENT
        ))
        return HomeWorkspaceSurface(panel, rows)
    }

    private fun createFriendsHeader(title: String): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(44)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(20), 0, dp(12), 0)
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = title
                setTextColor(activity.getColor(R.color.elon_text_primary))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 16f)
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            addView(FrameLayout(activity).apply {
                contentDescription = "添加好友"
                isClickable = true
                isFocusable = true
                foreground = selectableForeground()
                setOnClickListener { showAddFriendDialog() }
                addView(ImageView(activity).apply {
                    importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                    scaleType = ImageView.ScaleType.FIT_CENTER
                    setImageResource(R.drawable.ic_top_add_plus_custom)
                }, FrameLayout.LayoutParams(dp(28), dp(28), Gravity.CENTER))
            }, LinearLayout.LayoutParams(dp(48), dp(44)))
        }
    }

    private fun updateFriendsPanelMinimumHeight(root: LinearLayout, panel: FrameLayout) {
        panel.minimumHeight = dp(FRIENDS_PANEL_FALLBACK_HEIGHT_DP)
        panel.post {
            val viewport = (root.parent as? ScrollView)?.height ?: return@post
            panel.minimumHeight = maxOf(
                dp(FRIENDS_PANEL_FALLBACK_HEIGHT_DP),
                viewport - dp(PROJECT_SECTION_HEIGHT_DP)
            )
        }
    }

    private companion object {
        const val PROJECT_SECTION_HEIGHT_DP = 153
        const val PROJECT_HEADER_HEIGHT_DP = 44
        const val PROJECT_ITEM_WIDTH_DP = 72
        const val PROJECT_ITEM_HEIGHT_DP = 88
        const val PROJECT_ADD_GAP_DP = 8
        const val FRIENDS_PANEL_FALLBACK_HEIGHT_DP = 555
    }
}
