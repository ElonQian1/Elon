package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.text.InputFilter
import android.text.InputType
import android.view.Gravity
import android.view.View
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity

internal class ProjectSpacePostComposer(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> android.graphics.drawable.Drawable?,
    private val onBack: () -> Unit,
    private val onPickLocalImage: (ProjectSpaceSummary, (Result<String>) -> Unit) -> Unit,
    private val onSubmit: (
        channel: ProjectChannel,
        title: String,
        body: String,
        onComplete: (Result<Unit>) -> Unit
    ) -> Unit
) {
    fun render(
        container: LinearLayout,
        space: ProjectSpace,
        channels: List<ProjectChannel>,
        initialChannel: ProjectChannel? = null
    ) {
        container.removeAllViews()
        val selectableChannels = channels.filter { it.isProjectSpaceFeedChannel() }
        var selectedChannel = initialChannel?.takeIf { it.isProjectSpaceFeedChannel() }
        var visibilityLabel = "全场景可见"
        var imageUrl: String? = null

        val titleInput = postEditText(
            hintText = "请输入完整帖子标题(5-31个字)",
            minLinesValue = 1,
            maxLinesValue = 2,
            maxLength = 31
        ).apply {
            textSize = 15f
        }
        val bodyInput = postEditText(
            hintText = "请输入正文（建议200-2000字）",
            minLinesValue = 5,
            maxLinesValue = 11,
            maxLength = 2000
        )
        val imageTile = imageTile()
        val topicValue = rowValue(selectedChannel?.let(::projectSpaceTopicLabel)
            ?: "选择话题类型会让内容有更多曝光哦")
        val visibilityValue = rowValue(visibilityLabel)
        val submitButton = submitButton()

        container.addView(backRow())
        container.addView(titleInput)
        container.addView(divider())
        container.addView(bodyInput)
        container.addView(imageTile)
        container.addView(divider())
        container.addView(infoRow("⊘", space.project.name.ifBlank { "项目文档" }, null) {})
        container.addView(infoRow("#", "添加话题", topicValue) {
            showTopicDialog(selectableChannels, selectedChannel) { channel ->
                selectedChannel = channel
                topicValue.text = projectSpaceTopicLabel(channel)
            }
        })
        container.addView(infoRow("◉", "设置展示范围", visibilityValue) {
            showVisibilityDialog(visibilityLabel) { next ->
                visibilityLabel = next
                visibilityValue.text = next
            }
        })
        container.addView(infoRow("!", "内容声明", null) {
            showContentStatement()
        })
        container.addView(submitButton)

        imageTile.setOnClickListener {
            showImagePickerDialog(
                current = imageUrl,
                onUploadLocal = {
                    imageTile.text = "正在上传图片..."
                    imageTile.textSize = 13f
                    onPickLocalImage(space.project) { result ->
                        result.onSuccess { next ->
                            imageUrl = next
                            setImageTileState(imageTile, next, "已上传本地图片")
                        }.onFailure {
                            setImageTileState(imageTile, imageUrl, "已添加图片")
                            toast(it.message ?: "图片上传失败")
                        }
                    }
                },
                onPasteUrl = {
                    showImageUrlDialog(imageUrl) { next ->
                        imageUrl = next
                        setImageTileState(imageTile, next, "已添加图片链接")
                    }
                },
                onRemove = {
                    imageUrl = null
                    setImageTileState(imageTile, null, "已添加图片")
                }
            )
        }

        submitButton.setOnClickListener {
            val title = titleInput.text?.toString()?.trim().orEmpty()
            val body = bodyInput.text?.toString()?.trim().orEmpty()
            val channel = selectedChannel ?: selectableChannels.firstOrNull()
            when {
                channel == null -> toast("当前项目没有可发帖的话题")
                title.length !in 5..31 -> toast("帖子标题需要 5-31 个字")
                body.isBlank() -> toast("请输入正文内容")
                else -> {
                    val finalBody = buildPostBody(body, imageUrl, visibilityLabel)
                    submitButton.isEnabled = false
                    submitButton.text = "发布中..."
                    onSubmit(channel, title, finalBody) { result ->
                        submitButton.isEnabled = true
                        submitButton.text = "发布"
                        result.onFailure { toast(it.message ?: "发布失败") }
                    }
                }
            }
        }
    }

    private fun backRow(): TextView {
        return TextView(activity).apply {
            text = "‹ 项目空间"
            textSize = 15f
            gravity = Gravity.CENTER_VERTICAL
            setTextColor(Color.parseColor("#DDE8FC"))
            setPadding(dp(20), dp(14), dp(20), dp(14))
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onBack() }
        }
    }

    private fun postEditText(
        hintText: String,
        minLinesValue: Int,
        maxLinesValue: Int,
        maxLength: Int
    ): EditText {
        return EditText(activity).apply {
            hint = hintText
            minLines = minLinesValue
            maxLines = maxLinesValue
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
            filters = arrayOf(InputFilter.LengthFilter(maxLength))
            setTextColor(Color.parseColor("#F2F5FA"))
            setHintTextColor(Color.parseColor("#6F7785"))
            background = ColorDrawable(Color.TRANSPARENT)
            gravity = Gravity.TOP or Gravity.START
            setPadding(dp(14), dp(16), dp(14), dp(14))
            textSize = 14f
        }
    }

    private fun imageTile(): TextView {
        return TextView(activity).apply {
            text = "+"
            textSize = 34f
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor("#8C8C8C"))
            background = roundedBackground("#1E1E1E", 6)
            isClickable = true
            foreground = selectableForeground()
            layoutParams = LinearLayout.LayoutParams(dp(96), dp(96)).apply {
                setMargins(dp(14), dp(72), dp(14), dp(24))
            }
        }
    }

    private fun setImageTileState(imageTile: TextView, imageUrl: String?, label: String) {
        val cleanUrl = imageUrl?.trim()?.takeIf { it.isNotBlank() }
        if (cleanUrl == null) {
            imageTile.text = "+"
            imageTile.textSize = 34f
            imageTile.setTextColor(Color.parseColor("#8C8C8C"))
        } else {
            imageTile.text = "$label\n${cleanUrl.take(42)}"
            imageTile.textSize = 12f
            imageTile.setTextColor(Color.parseColor("#DDE8FC"))
        }
    }

    private fun infoRow(
        iconText: String,
        titleText: String,
        valueText: TextView?,
        onClick: () -> Unit
    ): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            minimumHeight = dp(46)
            setPadding(dp(14), dp(6), dp(14), dp(6))
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
            addView(TextView(activity).apply {
                text = iconText
                textSize = 17f
                gravity = Gravity.CENTER
                setTextColor(Color.parseColor("#A6AFBD"))
            }, LinearLayout.LayoutParams(dp(24), LinearLayout.LayoutParams.WRAP_CONTENT).apply {
                marginEnd = dp(8)
            })
            addView(TextView(activity).apply {
                text = titleText
                textSize = 14f
                setTextColor(Color.parseColor("#A6AFBD"))
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            valueText?.let {
                addView(it, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1.1f))
            }
            addView(TextView(activity).apply {
                text = "›"
                textSize = 20f
                gravity = Gravity.CENTER
                setTextColor(Color.parseColor("#6F7785"))
            }, LinearLayout.LayoutParams(dp(24), LinearLayout.LayoutParams.WRAP_CONTENT))
        }
    }

    private fun rowValue(textValue: String): TextView {
        return TextView(activity).apply {
            text = textValue
            textSize = 12f
            gravity = Gravity.END or Gravity.CENTER_VERTICAL
            setTextColor(Color.parseColor("#6F7785"))
            maxLines = 1
            ellipsize = android.text.TextUtils.TruncateAt.END
        }
    }

    private fun submitButton(): TextView {
        return TextView(activity).apply {
            text = "发布"
            textSize = 15f
            gravity = Gravity.CENTER
            setTypeface(typeface, Typeface.BOLD)
            setTextColor(Color.parseColor("#07120A"))
            background = roundedBackground("#58BE6A", 7)
            isClickable = true
            foreground = selectableForeground()
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(44)
            ).apply {
                setMargins(dp(14), dp(28), dp(14), dp(32))
            }
        }
    }

    private fun showTopicDialog(
        channels: List<ProjectChannel>,
        selected: ProjectChannel?,
        onSelected: (ProjectChannel) -> Unit
    ) {
        if (channels.isEmpty()) {
            toast("当前项目没有可用话题")
            return
        }
        val labels = channels.map(::projectSpaceTopicLabel).toTypedArray()
        val checked = channels.indexOfFirst { it.id == selected?.id }.takeIf { it >= 0 } ?: -1
        AlertDialog.Builder(activity)
            .setTitle("选择话题类型")
            .setSingleChoiceItems(labels, checked) { dialog, which ->
                onSelected(channels[which])
                dialog.dismiss()
            }
            .show()
    }

    private fun showVisibilityDialog(current: String, onSelected: (String) -> Unit) {
        val labels = arrayOf("全场景可见", "仅项目成员可见")
        AlertDialog.Builder(activity)
            .setTitle("设置展示范围")
            .setSingleChoiceItems(labels, labels.indexOf(current).coerceAtLeast(0)) { dialog, which ->
                onSelected(labels[which])
                dialog.dismiss()
            }
            .show()
    }

    private fun showImagePickerDialog(
        current: String?,
        onUploadLocal: () -> Unit,
        onPasteUrl: () -> Unit,
        onRemove: () -> Unit
    ) {
        val labels = if (current.isNullOrBlank()) {
            arrayOf("上传本地图片", "粘贴图片 URL")
        } else {
            arrayOf("上传本地图片", "粘贴图片 URL", "移除图片")
        }
        AlertDialog.Builder(activity)
            .setTitle("添加图片")
            .setItems(labels) { dialog, which ->
                when (labels[which]) {
                    "上传本地图片" -> onUploadLocal()
                    "粘贴图片 URL" -> onPasteUrl()
                    "移除图片" -> onRemove()
                }
                dialog.dismiss()
            }
            .show()
    }

    private fun showImageUrlDialog(current: String?, onSelected: (String?) -> Unit) {
        val input = EditText(activity).apply {
            hint = "粘贴图片 URL（可选）"
            setText(current.orEmpty())
            setTextColor(Color.parseColor("#F2F5FA"))
            setHintTextColor(Color.parseColor("#6F7785"))
            setPadding(dp(12), dp(10), dp(12), dp(10))
            background = roundedBackground("#181B20", 8)
            setSelection(text?.length ?: 0)
        }
        AlertDialog.Builder(activity)
            .setTitle("粘贴图片 URL")
            .setView(LinearLayout(activity).apply {
                setPadding(dp(20), dp(8), dp(20), 0)
                addView(input)
            })
            .setNegativeButton("移除") { _, _ -> onSelected(null) }
            .setPositiveButton("保存") { _, _ ->
                onSelected(input.text?.toString()?.trim()?.takeIf { it.isNotBlank() })
            }
            .show()
    }

    private fun showContentStatement() {
        AlertDialog.Builder(activity)
            .setTitle("内容声明")
            .setMessage("请发布与当前项目相关的讨论、需求、意见或问题反馈。上传本地图片或包含图片链接时，请确认图片可公开展示且不含敏感信息。")
            .setPositiveButton("知道了", null)
            .show()
    }

    private fun buildPostBody(body: String, imageUrl: String?, visibilityLabel: String): String {
        return buildString {
            append(body.trim())
            imageUrl?.trim()?.takeIf { it.isNotBlank() }?.let {
                append("\n\n![图片](").append(it).append(")")
            }
            append("\n\n展示范围：").append(visibilityLabel)
        }
    }

    private fun divider(): View {
        return View(activity).apply {
            setBackgroundColor(Color.parseColor("#1E2126"))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                1
            ).apply {
                marginStart = dp(14)
                marginEnd = dp(14)
            }
        }
    }

    private fun roundedBackground(colorHex: String, radiusDp: Int): GradientDrawable {
        return GradientDrawable().apply {
            setColor(Color.parseColor(colorHex))
            cornerRadius = dp(radiusDp).toFloat()
        }
    }

    private fun toast(message: String) {
        Toast.makeText(activity, message, Toast.LENGTH_SHORT).show()
    }
}
