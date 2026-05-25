package com.elon.app

import android.content.pm.PackageManager
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity

internal class MainLifecycleEdgeActions(
    private val activity: AppCompatActivity,
    private val speechPermissionRequest: Int,
    private val notificationPermissionRequest: Int,
    private val stopStageHintShimmer: () -> Unit,
    private val cancelHomeRowShimmer: () -> Unit,
    private val destroySpeechInput: () -> Unit,
    private val isTaskWorkReceiverRegistered: () -> Boolean,
    private val unregisterTaskWorkReceiver: () -> Unit
) {
    fun onRequestPermissionsResult(requestCode: Int, grantResults: IntArray) {
        if (requestCode == speechPermissionRequest) {
            val granted = grantResults.firstOrNull() == PackageManager.PERMISSION_GRANTED
            Toast.makeText(
                activity,
                if (granted) "已开启语音权限，请按住说话" else "需要麦克风权限才能语音转文字",
                Toast.LENGTH_SHORT
            ).show()
        } else if (requestCode == notificationPermissionRequest) {
            val granted = grantResults.firstOrNull() == PackageManager.PERMISSION_GRANTED
            if (!granted) {
                Toast.makeText(activity, "需要通知权限才能显示任务完成和应用更新提醒", Toast.LENGTH_SHORT).show()
            }
        }
    }

    fun onDestroy() {
        stopStageHintShimmer()
        cancelHomeRowShimmer()
        destroySpeechInput()
        if (isTaskWorkReceiverRegistered()) {
            unregisterTaskWorkReceiver()
        }
    }
}
