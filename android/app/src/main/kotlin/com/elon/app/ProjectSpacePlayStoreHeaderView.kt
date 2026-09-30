package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.HorizontalScrollView
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.graphics.drawable.RoundedBitmapDrawableFactory

internal class ProjectSpacePlayStoreHeaderView(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> android.graphics.drawable.Drawable?,
    private val openProjectMembers: () -> Unit,
    private val openChannel: (ProjectChannel) -> Unit,
    private val openProjectDescription: (ProjectSpace) -> Unit,
    private val joinProject: () -> Unit,
    private val openProjectDocuments: () -> Unit,
    private val openProjectResources: () -> Unit,
    private val projectApkActionLabel: () -> String,
    private val downloadProjectApk: () -> Unit,
    private val replaceProjectPreviewImage: (ProjectSpace, Int) -> Unit
) {
    private val colors = MobileColors(activity)

    fun render(space: ProjectSpace, previewImages: List<String?>): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(colors.surface)
            addView(ProjectSpaceOverviewView(activity, dp, openChannel, openProjectMembers, joinProject).render(space))
            addView(ProjectIntroductionView(activity, dp, openProjectMembers, openChannel) { openProjectDescription(space) }.render(space))
            addView(TextView(activity).apply {
                text = "资料与版本"
                textSize = 20f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(colors.text)
                setPadding(dp(24), dp(20), dp(24), dp(8))
            })
            addView(resourceAction("项目文档", openProjectDocuments))
            addView(resourceAction("项目资源", openProjectResources))
            if (!space.latestApkUrl.isNullOrBlank()) {
                addView(resourceAction(projectApkActionLabel().ifBlank { "安装项目 APK" }, downloadProjectApk))
            }
            addView(previewStrip(space, previewImages), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, dp(167)
            ).apply { topMargin = dp(16); bottomMargin = dp(20) })
        }
    }

    private fun resourceAction(title: String, onClick: () -> Unit) = TextView(activity).apply {
        text = "$title ›"
        textSize = 16f
        minimumHeight = dp(48)
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(24), dp(12), dp(24), dp(12))
        setTextColor(colors.primary)
        isClickable = true
        isFocusable = true
        foreground = selectableForeground()
        setOnClickListener { onClick() }
    }
    private fun previewStrip(space: ProjectSpace, previewImages: List<String?>): HorizontalScrollView {
        return HorizontalScrollView(activity).apply {
            isHorizontalScrollBarEnabled = false
            overScrollMode = View.OVER_SCROLL_NEVER
            clipToPadding = false
            setPadding(dp(24), 0, dp(24), 0)
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                (0 until PREVIEW_SLOT_COUNT).forEach { index ->
                    addView(previewCard(space, index, previewImages.getOrNull(index)), LinearLayout.LayoutParams(dp(88), dp(167)).apply {
                        if (index < 3) marginEnd = dp(13)
                    })
                }
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            ))
        }
    }

    private fun previewCard(space: ProjectSpace, index: Int, source: String?): FrameLayout {
        val cleanSource = source.cleanProjectSpaceDisplayName()
        val editable = canEditProjectDescription(space.project.role)
        return FrameLayout(activity).apply {
            background = GradientDrawable().apply { setColor(this@ProjectSpacePlayStoreHeaderView.colors.container); cornerRadius = dp(8).toFloat() }
            clipToPadding = true
            contentDescription = if (cleanSource == null) "应用图片占位" else "应用图片 ${index + 1}"
            if (editable) {
                isLongClickable = true
                setOnLongClickListener {
                    replaceProjectPreviewImage(space, index)
                    true
                }
            }
            if (cleanSource == null) {
                addView(ImageView(activity).apply {
                    setImageResource(R.drawable.ic_project_preview_placeholder)
                    scaleType = ImageView.ScaleType.CENTER_INSIDE
                    adjustViewBounds = false
                }, FrameLayout.LayoutParams(dp(34), dp(34), Gravity.CENTER))
            } else {
                addView(ImageView(activity).apply {
                    tag = cleanSource
                    scaleType = ImageView.ScaleType.CENTER
                    setBackgroundColor(colors.surface)
                    setImageResource(R.drawable.ic_project_preview_placeholder)
                    setOnClickListener { openPreviewImage(cleanSource) }
                    if (editable) {
                        isLongClickable = true
                        setOnLongClickListener {
                            replaceProjectPreviewImage(space, index)
                            true
                        }
                    }
                    ChatImagePreviewLoader.cached(cleanSource)?.let { bitmap ->
                        scaleType = ImageView.ScaleType.CENTER_CROP
                        setImageDrawable(RoundedBitmapDrawableFactory.create(resources, bitmap).apply {
                            cornerRadius = dp(8).toFloat()
                            setAntiAlias(true)
                        })
                    } ?: ChatImagePreviewLoader.load(activity, cleanSource) { bitmap ->
                        post {
                            if (tag == cleanSource) {
                                scaleType = ImageView.ScaleType.CENTER_CROP
                                setImageDrawable(RoundedBitmapDrawableFactory.create(resources, bitmap).apply {
                                    cornerRadius = dp(8).toFloat()
                                    setAntiAlias(true)
                                })
                            }
                        }
                    }
                }, FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT
                ))
            }
        }
    }

    private fun openPreviewImage(source: String) {
        val attachment = if (source.startsWith("http://", true) || source.startsWith("https://", true)) {
            ChatAttachment(kind = "image", displayName = "应用图片", mimeType = "image/*", url = source)
        } else {
            ChatAttachment(kind = "image", displayName = "应用图片", mimeType = "image/*", localPath = source)
        }
        ChatImageViewer.show(activity, attachment)
    }

    private companion object { const val PREVIEW_SLOT_COUNT = 4 }
}
