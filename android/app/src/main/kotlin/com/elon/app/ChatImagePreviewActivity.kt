package com.elon.app

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.view.Gravity
import android.view.View
import android.view.Window
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import java.io.File
import kotlin.math.max

internal class ChatImagePreviewActivity : AppCompatActivity() {
    private lateinit var heroImage: ImageView
    private lateinit var thumbImage: ImageView
    private var currentPath = ""
    private var currentName = "图片"
    private var currentWidth = 0
    private var currentHeight = 0
    private var keepCurrentFile = false

    private val editLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode != Activity.RESULT_OK) return@registerForActivityResult
        val data = result.data ?: return@registerForActivityResult
        val nextPath = data.getStringExtra(ChatImageEditActivity.EXTRA_OUTPUT_PATH).orEmpty()
        val nextFile = File(nextPath)
        if (!nextFile.exists()) {
            Toast.makeText(this, "编辑后的图片已失效", Toast.LENGTH_SHORT).show()
            return@registerForActivityResult
        }
        val previous = currentPath
        currentPath = nextPath
        currentName = data.getStringExtra(ChatImageEditActivity.EXTRA_OUTPUT_NAME) ?: nextFile.name
        currentWidth = data.getIntExtra(ChatImageEditActivity.EXTRA_OUTPUT_WIDTH, 0)
        currentHeight = data.getIntExtra(ChatImageEditActivity.EXTRA_OUTPUT_HEIGHT, 0)
        if (previous.isNotBlank() && previous != currentPath) runCatching { File(previous).delete() }
        renderImages()
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        requestWindowFeature(Window.FEATURE_NO_TITLE)
        window.statusBarColor = Color.BLACK
        window.navigationBarColor = Color.BLACK
        currentPath = intent.getStringExtra(EXTRA_INPUT_PATH).orEmpty()
        currentName = intent.getStringExtra(EXTRA_DISPLAY_NAME).orEmpty().ifBlank { "图片" }
        if (!File(currentPath).exists()) {
            Toast.makeText(this, "图片读取失败", Toast.LENGTH_SHORT).show()
            finish()
            return
        }
        setContentView(FrameLayout(this).apply {
            setBackgroundColor(Color.BLACK)
            heroImage = ImageView(context).apply {
                layoutParams = FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT
                ).apply {
                    topMargin = dp(PREVIEW_IMAGE_TOP_MARGIN_DP)
                    bottomMargin = dp(PREVIEW_IMAGE_BOTTOM_MARGIN_DP)
                }
                adjustViewBounds = true
                scaleType = ImageView.ScaleType.FIT_CENTER
                contentDescription = "图片预览"
            }
            addView(heroImage)
            addView(topBar())
            addView(bottomPreviewTray())
        })
        renderImages()
    }

    private fun topBar(): View {
        return LinearLayout(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                dp(TOP_BAR_HEIGHT_DP),
                Gravity.TOP
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(24), 0, dp(20), 0)
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(0, dp(PRIMARY_CONTROL_HEIGHT_DP), 1f)
                gravity = Gravity.CENTER_VERTICAL or Gravity.START
                includeFontPadding = false
                text = "预览"
                setTextColor(getColor(R.color.elon_text_primary))
                textSize = 16f
            })
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(dp(112), dp(PRIMARY_CONTROL_HEIGHT_DP))
                background = roundedRect(getColor(R.color.elon_button_primary_bg), dp(24))
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = "添加 (1)"
                setTextColor(getColor(R.color.elon_button_primary_text))
                textSize = 16f
                setOnClickListener { finishWithImage() }
            })
        }
    }

    private fun bottomPreviewTray(): View {
        return FrameLayout(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                dp(BOTTOM_TRAY_HEIGHT_DP),
                Gravity.BOTTOM
            )
            setPadding(dp(24), 0, dp(24), dp(20))
            val thumbWrap = FrameLayout(context).apply {
                layoutParams = FrameLayout.LayoutParams(dp(112), dp(96), Gravity.START or Gravity.BOTTOM)
            }
            thumbImage = ImageView(context).apply {
                layoutParams = FrameLayout.LayoutParams(
                    dp(THUMB_SIZE_DP),
                    dp(THUMB_SIZE_DP),
                    Gravity.BOTTOM or Gravity.START
                )
                background = roundedRect(getColor(R.color.elon_surface_card), dp(6))
                clipToOutline = true
                scaleType = ImageView.ScaleType.CENTER_CROP
                contentDescription = "已选图片"
            }
            thumbWrap.addView(thumbImage)
            thumbWrap.addView(editButton())
            thumbWrap.addView(closeButton())
            addView(thumbWrap)
        }
    }

    private fun editButton(): ImageButton {
        return ImageButton(this).apply {
            layoutParams = FrameLayout.LayoutParams(dp(64), dp(48), Gravity.TOP or Gravity.START)
            background = ColorDrawable(Color.TRANSPARENT)
            contentDescription = "编辑图片"
            scaleType = ImageView.ScaleType.FIT_START
            setPadding(0, 0, dp(12), dp(8))
            setImageResource(R.drawable.ic_chat_image_edit_marker)
            setOnClickListener { openEditor() }
        }
    }

    private fun closeButton(): TextView {
        return TextView(this).apply {
            layoutParams = FrameLayout.LayoutParams(dp(48), dp(48), Gravity.TOP or Gravity.START).apply {
                leftMargin = dp(52)
                topMargin = dp(22)
            }
            background = ColorDrawable(Color.TRANSPARENT)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = "×"
            setTextColor(getColor(R.color.elon_text_primary))
            textSize = 22f
            contentDescription = "取消这张图片"
            setOnClickListener {
                setResult(Activity.RESULT_CANCELED)
                finish()
            }
        }
    }

    private fun openEditor() {
        if (!File(currentPath).exists()) {
            Toast.makeText(this, "图片读取失败", Toast.LENGTH_SHORT).show()
            return
        }
        editLauncher.launch(ChatImageEditActivity.createIntent(this, currentPath, currentName))
    }

    private fun finishWithImage() {
        val file = File(currentPath)
        if (!file.exists()) {
            Toast.makeText(this, "图片读取失败", Toast.LENGTH_SHORT).show()
            return
        }
        setResult(
            Activity.RESULT_OK,
            Intent().apply {
                putExtra(EXTRA_OUTPUT_PATH, currentPath)
                putExtra(EXTRA_OUTPUT_NAME, currentName)
                putExtra(EXTRA_OUTPUT_WIDTH, currentWidth)
                putExtra(EXTRA_OUTPUT_HEIGHT, currentHeight)
            }
        )
        keepCurrentFile = true
        finish()
    }

    override fun onDestroy() {
        if (!keepCurrentFile && currentPath.isNotBlank()) {
            runCatching { File(currentPath).delete() }
        }
        super.onDestroy()
    }

    private fun renderImages() {
        val bitmap = decodePreviewBitmap(currentPath)
        if (bitmap == null) {
            Toast.makeText(this, "图片读取失败", Toast.LENGTH_SHORT).show()
            return
        }
        if (currentWidth <= 0 || currentHeight <= 0) {
            currentWidth = bitmap.width
            currentHeight = bitmap.height
        }
        heroImage.setImageBitmap(bitmap)
        thumbImage.setImageBitmap(bitmap)
    }

    private fun decodePreviewBitmap(path: String): Bitmap? {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeFile(path, bounds)
        if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return null
        var sample = 1
        val maxPixels = 3_000_000
        while ((bounds.outWidth / sample) * (bounds.outHeight / sample) > maxPixels) {
            sample *= 2
        }
        val minSide = max(bounds.outWidth / sample, bounds.outHeight / sample)
        return BitmapFactory.decodeFile(
            path,
            BitmapFactory.Options().apply {
                inSampleSize = sample
                inDensity = minSide
                inTargetDensity = minSide
            }
        )
    }

    private fun roundedRect(color: Int, radius: Int): GradientDrawable {
        return GradientDrawable().apply {
            cornerRadius = radius.toFloat()
            setColor(color)
        }
    }

    private fun dp(value: Int): Int {
        return (value * resources.displayMetrics.density).toInt()
    }

    companion object {
        const val EXTRA_INPUT_PATH = "chat_image_preview_input_path"
        const val EXTRA_DISPLAY_NAME = "chat_image_preview_display_name"
        const val EXTRA_OUTPUT_PATH = "chat_image_preview_output_path"
        const val EXTRA_OUTPUT_NAME = "chat_image_preview_output_name"
        const val EXTRA_OUTPUT_WIDTH = "chat_image_preview_output_width"
        const val EXTRA_OUTPUT_HEIGHT = "chat_image_preview_output_height"
        private const val TOP_BAR_HEIGHT_DP = 64
        private const val PRIMARY_CONTROL_HEIGHT_DP = 48
        private const val PREVIEW_IMAGE_TOP_MARGIN_DP = 88
        private const val PREVIEW_IMAGE_BOTTOM_MARGIN_DP = 124
        private const val BOTTOM_TRAY_HEIGHT_DP = 116
        private const val THUMB_SIZE_DP = 64

        fun createIntent(context: Context, path: String, displayName: String): Intent {
            return Intent(context, ChatImagePreviewActivity::class.java).apply {
                putExtra(EXTRA_INPUT_PATH, path)
                putExtra(EXTRA_DISPLAY_NAME, displayName)
            }
        }
    }
}
