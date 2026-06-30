package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.SpannableString
import android.text.Spanned
import android.text.TextUtils
import android.text.style.ForegroundColorSpan
import android.util.TypedValue
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
    private val joinProject: () -> Unit,
    private val openProjectDocuments: () -> Unit,
    private val openProjectResources: () -> Unit,
    private val projectApkActionLabel: () -> String,
    private val downloadProjectApk: () -> Unit
) {
    fun render(space: ProjectSpace, postCount: Int): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(PLAY_BG))
            setPadding(0, dp(PLAY_CONTENT_TOP_DP), 0, dp(28))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )

            addView(projectAppHeader(space))
            addView(projectQuickActions(), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(36)
            ).apply {
                topMargin = dp(14)
            })
            addView(projectMetrics(space, postCount), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(68)
            ).apply {
                topMargin = dp(20)
            })
            addView(installButton(), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(40)
            ).apply {
                leftMargin = dp(24)
                topMargin = dp(28)
                rightMargin = dp(24)
            })
            addView(previewStrip(), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(167)
            ).apply {
                topMargin = dp(16)
            })
        }
    }

    private fun projectAppHeader(space: ProjectSpace): LinearLayout {
        val owner = space.members.firstOrNull { it.role.equals("owner", ignoreCase = true) }
            ?: space.members.firstOrNull()
        val ownerName = owner?.account?.takeIf { it.isNotBlank() } ?: "Elon Project"
        val subtitle = if (space.latestApkUrl.isNullOrBlank()) {
            "项目协作空间"
        } else {
            "包含可安装 APK"
        }
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.TOP
            setPadding(dp(24), 0, dp(24), 0)
            clipToPadding = false
            clipChildren = false

            addView(projectIcon(space.project), LinearLayout.LayoutParams(dp(72), dp(72)).apply {
                marginEnd = dp(24)
            })

            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER_VERTICAL
                addView(TextView(activity).apply {
                    text = space.project.name.ifBlank { "项目空间" }
                    setVisualTextSize(23)
                    includeFontPadding = true
                    setTextColor(Color.parseColor(PLAY_TEXT_PRIMARY))
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                })
                addView(TextView(activity).apply {
                    text = ownerName
                    setVisualTextSize(15)
                    includeFontPadding = true
                    typeface = Typeface.create("sans-serif-medium", Typeface.NORMAL)
                    setTextColor(Color.parseColor(PLAY_LINK))
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                }, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = dp(2)
                })
                addView(TextView(activity).apply {
                    text = subtitle
                    setVisualTextSize(12)
                    includeFontPadding = true
                    setTextColor(Color.parseColor(PLAY_TEXT_SECONDARY))
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                }, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = dp(1)
                })
            }, LinearLayout.LayoutParams(0, dp(72), 1f))

            addView(joinButton(space), LinearLayout.LayoutParams(dp(104), dp(48)).apply {
                marginStart = dp(18)
                topMargin = dp(12)
            })
        }
    }

    private fun joinButton(space: ProjectSpace): TextView {
        val visitor = isProjectSpaceVisitor(space.project.role)
        return TextView(activity).apply {
            text = if (visitor) "加入" else "成员"
            setVisualTextSize(19)
            includeFontPadding = false
            gravity = Gravity.CENTER
            typeface = Typeface.create("sans-serif-medium", Typeface.NORMAL)
            setTextColor(Color.parseColor(PLAY_TEXT_PRIMARY))
            background = roundedStrokeBackground(PLAY_BG, 8, "#5A5A5A", 1)
            isClickable = true
            foreground = selectableForeground()
            contentDescription = if (visitor) "加入项目" else "查看项目成员"
            setOnClickListener {
                if (visitor) joinProject() else openProjectMembers()
            }
        }
    }

    private fun projectQuickActions(): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(24), 0, dp(24), 0)
            addView(projectQuickActionButton("项目文档", "查看项目文档", openProjectDocuments), LinearLayout.LayoutParams(
                dp(104),
                LinearLayout.LayoutParams.MATCH_PARENT
            ).apply {
                marginEnd = dp(12)
            })
            addView(projectQuickActionButton("项目资源", "查看项目资源", openProjectResources), LinearLayout.LayoutParams(
                dp(104),
                LinearLayout.LayoutParams.MATCH_PARENT
            ))
        }
    }

    private fun projectQuickActionButton(
        label: String,
        description: String,
        onClick: () -> Unit
    ): TextView {
        return TextView(activity).apply {
            text = label
            textSize = 14f
            includeFontPadding = false
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor(PLAY_TEXT_PRIMARY))
            background = roundedStrokeBackground(PLAY_BG, 8, "#5A5A5A", 1)
            isClickable = true
            foreground = selectableForeground()
            contentDescription = description
            setOnClickListener { onClick() }
        }
    }

    private fun TextView.setVisualTextSize(sizeDp: Int) {
        setTextSize(TypedValue.COMPLEX_UNIT_DIP, sizeDp.toFloat())
    }

    private fun projectIcon(project: ProjectSpaceSummary): View {
        val bitmap = UserProfileStore.decodeAvatar(project.iconDataUrl.cleanProjectSpaceDisplayName())
        if (bitmap != null) {
            return ImageView(activity).apply {
                scaleType = ImageView.ScaleType.CENTER_CROP
                setImageDrawable(RoundedBitmapDrawableFactory.create(resources, bitmap).apply {
                    cornerRadius = dp(18).toFloat()
                    setAntiAlias(true)
                })
            }
        }
        return FrameLayout(activity).apply {
            background = roundedBackground("#FFFFFF", 18)
            addView(TextView(activity).apply {
                text = project.name.firstOrNull()?.toString() ?: "项"
                gravity = Gravity.CENTER
                includeFontPadding = false
                textSize = 20f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.WHITE)
                background = roundedBackground("#2EA7DF", 999)
            }, FrameLayout.LayoutParams(dp(50), dp(50), Gravity.CENTER))
        }
    }

    private fun projectMetrics(space: ProjectSpace, postCount: Int): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER
            setPadding(dp(24), 0, dp(24), 0)
            addView(metricCell(ratingLabel(), "${postCount.coerceAtLeast(0)}贴", "评价 ⓘ"), metricCellParams())
            addView(metricDivider(), LinearLayout.LayoutParams(dp(1), dp(26)))
            addView(metricCell(
                value = "成员 ${space.project.memberCount.coerceAtLeast(1)}",
                label = "协作",
                caption = "空间"
            ).apply {
                isClickable = true
                foreground = selectableForeground()
                contentDescription = if (isProjectSpaceVisitor(space.project.role)) {
                    "申请加入联合开发"
                } else {
                    "查看项目成员"
                }
                setOnClickListener {
                    if (isProjectSpaceVisitor(space.project.role)) joinProject() else openProjectMembers()
                }
            }, metricCellParams())
            addView(metricDivider(), LinearLayout.LayoutParams(dp(1), dp(26)))
            addView(metricCell("12+", "12 岁以上 ⓘ", ""), metricCellParams())
        }
    }

    private fun metricCell(value: CharSequence, label: String, caption: String): LinearLayout {
        val detail = listOf(label, caption)
            .filter { it.isNotBlank() }
            .joinToString(" ")
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            addView(TextView(activity).apply {
                text = value
                textSize = 16f
                includeFontPadding = false
                typeface = Typeface.create("sans-serif-medium", Typeface.NORMAL)
                setTextColor(Color.parseColor(PLAY_TEXT_PRIMARY))
                gravity = Gravity.CENTER
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            })
            addView(TextView(activity).apply {
                text = detail
                setVisualTextSize(12)
                includeFontPadding = true
                setTextColor(Color.parseColor(PLAY_TEXT_SECONDARY))
                gravity = Gravity.CENTER
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(7)
            })
        }
    }

    private fun ratingLabel(): SpannableString {
        return SpannableString("4.8★").apply {
            setSpan(
                ForegroundColorSpan(Color.parseColor(PLAY_LINK)),
                length - 1,
                length,
                Spanned.SPAN_EXCLUSIVE_EXCLUSIVE
            )
        }
    }

    private fun metricCellParams(): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
    }

    private fun metricDivider(): View {
        return View(activity).apply {
            setBackgroundColor(Color.parseColor(PLAY_DIVIDER))
        }
    }

    private fun installButton(): TextView {
        return TextView(activity).apply {
            text = projectApkActionLabel().ifBlank { "安装" }
            textSize = 14f
            includeFontPadding = false
            gravity = Gravity.CENTER
            typeface = Typeface.create("sans-serif-medium", Typeface.NORMAL)
            setTextColor(Color.parseColor(PLAY_INSTALL_TEXT))
            background = roundedBackground(PLAY_INSTALL_BG, 20)
            isClickable = true
            foreground = selectableForeground()
            contentDescription = "安装项目 APK"
            setOnClickListener { downloadProjectApk() }
        }
    }

    private fun previewStrip(): HorizontalScrollView {
        return HorizontalScrollView(activity).apply {
            isHorizontalScrollBarEnabled = false
            overScrollMode = View.OVER_SCROLL_NEVER
            clipToPadding = false
            setPadding(dp(24), 0, dp(24), 0)
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                listOf(
                    PreviewCard("Fast", "Simple, reliable builds", "#28AEEA", "#DDF5FF"),
                    PreviewCard("Powerful", "AI edits and packages", "#14AFC5", "#E4FBFF"),
                    PreviewCard("Secure", "Private project space", "#38B95C", "#F1FFF3"),
                    PreviewCard("Private", "Only your team sees it", "#168F8F", "#E9FFFF")
                ).forEachIndexed { index, card ->
                    addView(previewCard(card), LinearLayout.LayoutParams(dp(88), dp(167)).apply {
                        if (index < 3) marginEnd = dp(13)
                    })
                }
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            ))
        }
    }

    private fun previewCard(card: PreviewCard): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_HORIZONTAL
            setPadding(dp(8), dp(8), dp(8), dp(8))
            background = roundedBackground(card.bg, 8)
            addView(TextView(activity).apply {
                text = card.title
                textSize = 14f
                includeFontPadding = false
                typeface = Typeface.create("sans-serif-medium", Typeface.NORMAL)
                setTextColor(Color.WHITE)
                gravity = Gravity.CENTER
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            })
            addView(TextView(activity).apply {
                text = card.subtitle
                textSize = 6f
                includeFontPadding = false
                setTextColor(Color.parseColor(card.ink))
                gravity = Gravity.CENTER
                maxLines = 2
                ellipsize = TextUtils.TruncateAt.END
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(5)
            })
            addView(phoneMock(), LinearLayout.LayoutParams(dp(64), 0, 1f).apply {
                topMargin = dp(8)
            })
        }
    }

    private fun phoneMock(): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(5), dp(7), dp(5), dp(5))
            background = roundedBackground("#20384B", 10)
            repeat(7) { index ->
                addView(View(activity).apply {
                    background = roundedBackground(if (index % 3 == 0) "#2BBF92" else "#FFFFFF", 2)
                }, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    dp(if (index == 0) 10 else 7)
                ).apply {
                    if (index > 0) topMargin = dp(5)
                    if (index % 2 == 1) rightMargin = dp(14)
                })
            }
        }
    }

    private fun roundedBackground(colorHex: String, radiusDp: Int): GradientDrawable {
        return GradientDrawable().apply {
            setColor(Color.parseColor(colorHex))
            cornerRadius = dp(radiusDp).toFloat()
        }
    }

    private fun roundedStrokeBackground(
        colorHex: String,
        radiusDp: Int,
        strokeHex: String,
        strokeWidthDp: Int
    ): GradientDrawable {
        return GradientDrawable().apply {
            setColor(Color.parseColor(colorHex))
            cornerRadius = dp(radiusDp).toFloat()
            setStroke(dp(strokeWidthDp), Color.parseColor(strokeHex))
        }
    }

    private data class PreviewCard(
        val title: String,
        val subtitle: String,
        val bg: String,
        val ink: String
    )

    private companion object {
        const val PLAY_CONTENT_TOP_DP = 16
        const val PLAY_BG = "#131313"
        const val PLAY_TEXT_PRIMARY = "#E3E3E3"
        const val PLAY_TEXT_SECONDARY = "#C6C6C6"
        const val PLAY_LINK = "#A8C7FA"
        const val PLAY_DIVIDER = "#444444"
        const val PLAY_INSTALL_BG = "#AEC6F6"
        const val PLAY_INSTALL_TEXT = "#182E63"
    }
}
