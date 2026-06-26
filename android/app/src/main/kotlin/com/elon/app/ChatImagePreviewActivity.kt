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
                    topMargin = dp(96)
                    bottomMargin = dp(136)
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
                dp(96),
                Gravity.TOP
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(24), dp(28), dp(22), 0)
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(0, dp(52), 1f)
                gravity = Gravity.CENTER_VERTICAL or Gravity.START
                includeFontPadding = false
                text = "预览"
                setTextColor(Color.WHITE)
                textSize = 24f
            })
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(dp(136), dp(56))
                background = roundedRect("#58BE6A", dp(28))
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = "添加 (1)"
                setTextColor(Color.WHITE)
                textSize = 20f
                setOnClickListener { finishWithImage() }
            })
        }
    }

    private fun bottomPreviewTray(): View {
        return FrameLayout(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                dp(136),
                Gravity.BOTTOM
            )
            setPadding(dp(24), dp(8), dp(24), dp(24))
            val thumbWrap = FrameLayout(context).apply {
                layoutParams = FrameLayout.LayoutParams(dp(116), dp(104), Gravity.START or Gravity.BOTTOM)
            }
            thumbImage = ImageView(context).apply {
                layoutParams = FrameLayout.LayoutParams(dp(76), dp(76), Gravity.BOTTOM or Gravity.START)
                background = roundedRect("#1A1A1A", dp(8))
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
            layoutParams = FrameLayout.LayoutParams(dp(88), dp(42), Gravity.TOP or Gravity.START).apply {
                leftMargin = dp(-2)
            }
            background = ColorDrawable(Color.TRANSPARENT)
            contentDescription = "编辑图片"
            scaleType = ImageView.ScaleType.FIT_CENTER
            setPadding(0, 0, 0, 0)
            setImageResource(R.drawable.ic_chat_image_edit_marker)
            setOnClickListener { openEditor() }
        }
    }

    private fun closeButton(): TextView {
        return TextView(this).apply {
            layoutParams = FrameLayout.LayoutParams(dp(32), dp(32), Gravity.TOP or Gravity.END).apply {
                topMargin = dp(30)
                rightMargin = dp(20)
            }
            background = roundedOval("#CC000000")
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = "×"
            setTextColor(Color.WHITE)
            textSize = 25f
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

    private fun roundedRect(color: String, radius: Int): GradientDrawable {
        return GradientDrawable().apply {
            cornerRadius = radius.toFloat()
            setColor(Color.parseColor(color))
        }
    }

    private fun roundedOval(color: String): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.OVAL
            setColor(Color.parseColor(color))
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

        fun createIntent(context: Context, path: String, displayName: String): Intent {
            return Intent(context, ChatImagePreviewActivity::class.java).apply {
                putExtra(EXTRA_INPUT_PATH, path)
                putExtra(EXTRA_DISPLAY_NAME, displayName)
            }
        }
    }
}
