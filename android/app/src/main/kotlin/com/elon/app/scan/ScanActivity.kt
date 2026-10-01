package com.elon.app.scan

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.hardware.Camera
import android.net.Uri
import android.os.Bundle
import android.provider.Settings
import android.view.Gravity
import android.view.View
import android.view.WindowManager
import android.widget.LinearLayout
import android.widget.TextView
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import com.elon.app.R
import com.elon.app.elonColor
import com.google.android.material.button.MaterialButton
import com.journeyapps.barcodescanner.BarcodeCallback
import com.journeyapps.barcodescanner.BarcodeResult
import com.journeyapps.barcodescanner.BarcodeView
import com.journeyapps.barcodescanner.CameraPreview
import com.journeyapps.barcodescanner.DefaultDecoderFactory
import kotlin.concurrent.thread

/** Owns camera permission and lifecycle; decoding and result actions live in separate modules. */
class ScanActivity : AppCompatActivity() {
    private lateinit var preview: BarcodeView
    private lateinit var status: TextView
    private lateinit var torchButton: MaterialButton
    private lateinit var settingsButton: MaterialButton
    private var active = false
    private var cameraWanted = false
    private var processing = false
    private var choosingImage = false
    private var showingResult = false
    private var torch = false
    private var cameraId = -1
    private var imageRevision = 0
    private var results = emptyList<String>()
    private val permission = registerForActivityResult(ActivityResultContracts.RequestPermission()) { allowed ->
        cameraWanted = allowed
        settingsButton.visibility = if (allowed) View.GONE else View.VISIBLE
        if (allowed) startCamera() else status.text = "相机未获授权，可在权限设置中允许相机，也可从图片识别"
    }
    private val imagePicker = registerForActivityResult(ActivityResultContracts.GetContent()) { uri ->
        choosingImage = false
        if (uri == null) { status.text = "已取消选图，可继续扫码"; return@registerForActivityResult }
        processing = true; val revision = ++imageRevision
        status.text = "正在本地识别图片…"
        thread(name = "scan-image") {
            val decoded = runCatching { ScanImageDecoder.decode(applicationContext, uri) }
            runOnUiThread {
                if (isDestroyed || isFinishing || revision != imageRevision) return@runOnUiThread
                processing = false
                decoded.fold(onSuccess = { values ->
                    if (values.isEmpty()) status.text = "未识别到二维码或条码，请尝试清晰原图" else showResults(values)
                }, onFailure = { status.text = it.message ?: "图片识别失败，请重新选择" })
            }
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        WindowCompat.setDecorFitsSystemWindows(window, false)
        cameraId = savedInstanceState?.getInt("cameraId", -1) ?: -1
        results = savedInstanceState?.getStringArrayList("results")?.toList().orEmpty()
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(elonColor(R.color.elon_bg_app))
        }
        val header = LinearLayout(this).apply { gravity = Gravity.CENTER_VERTICAL }
        header.addView(button("返回") { finish() })
        header.addView(TextView(this).apply { text = "扫一扫"; textSize = 22f; setTextColor(elonColor(R.color.elon_text_primary)) })
        root.addView(header)
        preview = BarcodeView(this).apply {
            contentDescription = "扫码相机预览"
            decoderFactory = DefaultDecoderFactory(null, null, "UTF-8", 2)
            addStateListener(object : CameraPreview.StateListener {
                override fun previewSized() = Unit
                override fun previewStarted() { torchButton.isEnabled = packageManager.hasSystemFeature(PackageManager.FEATURE_CAMERA_FLASH) && !frontCamera() }
                override fun previewStopped() = Unit
                override fun cameraClosed() = Unit
                override fun cameraError(error: Exception) {
                    cameraWanted = false; stopCamera()
                    status.text = "相机无法使用或被占用，请重试或选择图片识别"
                }
            })
        }
        root.addView(preview, LinearLayout.LayoutParams(-1, 0, 1f))
        status = TextView(this).apply {
            text = "将二维码或常见条码放入画面；也可从图片识别"
            setTextColor(elonColor(R.color.elon_text_primary)); textSize = 15f
            setPadding(dp(16), dp(12), dp(16), dp(12)); accessibilityLiveRegion = View.ACCESSIBILITY_LIVE_REGION_POLITE
        }
        root.addView(status)
        root.addView(LinearLayout(this).apply {
            addView(button("开始 / 继续扫码") { requestCamera() }, LinearLayout.LayoutParams(0, -2, 1f))
            addView(button("从图片识别") {
                cameraWanted = false; stopCamera(); choosingImage = true; ++imageRevision; processing = false
                imagePicker.launch("image/*")
            }, LinearLayout.LayoutParams(0, -2, 1f))
        })
        root.addView(LinearLayout(this).apply {
            torchButton = button("打开手电筒") {
                torch = !torch; preview.setTorch(torch); torchButton.text = if (torch) "关闭手电筒" else "打开手电筒"
            }.apply { isEnabled = false }
            addView(torchButton, LinearLayout.LayoutParams(0, -2, 1f))
            addView(button("切换相机") {
                stopCamera(); cameraId = (resolvedCameraId() + 1) % Camera.getNumberOfCameras(); cameraWanted = true; startCamera()
            }.apply { isEnabled = Camera.getNumberOfCameras() > 1 }, LinearLayout.LayoutParams(0, -2, 1f))
        })
        settingsButton = button("相机权限设置") { startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.parse("package:$packageName"))) }.apply { visibility = View.GONE }
        root.addView(settingsButton)
        ViewCompat.setOnApplyWindowInsetsListener(root) { view, insets ->
            val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars()); view.setPadding(bars.left, bars.top, bars.right, bars.bottom); insets
        }
        setContentView(root)
        if (results.isEmpty()) cameraWanted = true
    }

    override fun onResume() {
        super.onResume(); active = true
        if (results.isNotEmpty() && !showingResult) showResults(results)
        else if (cameraWanted && hasPermission()) startCamera()
        else if (cameraWanted) { cameraWanted = false; permission.launch(Manifest.permission.CAMERA) }
    }
    override fun onPause() { active = false; stopCamera(); super.onPause() }
    override fun onDestroy() { ++imageRevision; super.onDestroy() }
    override fun onSaveInstanceState(outState: Bundle) {
        outState.putInt("cameraId", cameraId); outState.putStringArrayList("results", ArrayList(results)); super.onSaveInstanceState(outState)
    }
    private fun requestCamera() {
        ++imageRevision; processing = false; results = emptyList(); cameraWanted = true
        if (hasPermission()) startCamera() else permission.launch(Manifest.permission.CAMERA)
    }
    private fun startCamera() {
        if (!active || processing || choosingImage || showingResult || !hasPermission()) return
        settingsButton.visibility = View.GONE
        preview.cameraSettings = preview.cameraSettings.apply { requestedCameraId = cameraId }
        preview.decodeSingle(object : BarcodeCallback {
            override fun barcodeResult(result: BarcodeResult) {
                if (active && cameraWanted && !showingResult) showResults(listOf(result.text))
            }
        })
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        preview.resume(); status.text = "将二维码或条码放入画面，识别成功后会暂停"
    }
    private fun stopCamera() {
        if (::preview.isInitialized) { preview.setTorch(false); preview.pause() }
        torch = false
        if (::torchButton.isInitialized) { torchButton.text = "打开手电筒"; torchButton.isEnabled = false }
        window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
    }
    private fun showResults(values: List<String>) {
        results = values.distinct().take(8); cameraWanted = false; stopCamera()
        status.text = "识别成功，选择结果操作或继续扫码"
        if (!active || showingResult) return
        showingResult = true
        ScanResultDialog.show(this, results) { showingResult = false; results = emptyList() }
    }
    private fun hasPermission() = ContextCompat.checkSelfPermission(this, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED
    private fun resolvedCameraId(): Int {
        if (cameraId >= 0) return cameraId
        return (0 until Camera.getNumberOfCameras()).firstOrNull { id -> Camera.CameraInfo().also { Camera.getCameraInfo(id, it) }.facing == Camera.CameraInfo.CAMERA_FACING_BACK } ?: 0
    }
    private fun frontCamera() = runCatching { Camera.CameraInfo().also { Camera.getCameraInfo(resolvedCameraId(), it) }.facing == Camera.CameraInfo.CAMERA_FACING_FRONT }.getOrDefault(false)
    private fun button(label: String, action: () -> Unit) = MaterialButton(this).apply { text = label; contentDescription = label; minHeight = dp(48); setOnClickListener { action() } }
    private fun dp(value: Int) = (value * resources.displayMetrics.density).toInt()
}
