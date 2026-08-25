package com.elon.app

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat

class WebChatRealtimeVoiceBackgroundService : Service() {
    private val audioManager by lazy(LazyThreadSafetyMode.NONE) {
        getSystemService(AudioManager::class.java)
    }
    private val audioFocusRequest by lazy(LazyThreadSafetyMode.NONE) {
        AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN_TRANSIENT)
            .setAudioAttributes(
                AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_VOICE_COMMUNICATION)
                    .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
                    .build(),
            )
            .setOnAudioFocusChangeListener(::onAudioFocusChanged)
            .build()
    }
    private val overlay by lazy(LazyThreadSafetyMode.NONE) {
        WebChatRealtimeVoiceSystemOverlay(
            context = this,
            onPauseResume = { if (paused) requestResume(USER) else requestPause(USER) },
            onOpenApp = ::openApp,
            onHangUp = ::requestHangUp,
        )
    }
    private var active = false
    private var hostVisible = true
    private var paused = false
    private var userPaused = false
    private var mediaPaused = false
    private var status = WebChatRealtimeVoiceBackgroundStatus.CONNECTING
    private var detail = "正在连接语音"

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            WebChatRealtimeVoiceBackgroundProtocol.ACTION_START -> start(intent)
            WebChatRealtimeVoiceBackgroundProtocol.ACTION_UPDATE -> update(intent)
            WebChatRealtimeVoiceBackgroundProtocol.ACTION_HOST_VISIBILITY -> {
                hostVisible = intent.getBooleanExtra(
                    WebChatRealtimeVoiceBackgroundProtocol.EXTRA_HOST_VISIBLE,
                    true,
                )
                render()
            }
            WebChatRealtimeVoiceBackgroundProtocol.ACTION_PAUSE -> requestPause(USER)
            WebChatRealtimeVoiceBackgroundProtocol.ACTION_RESUME -> requestResume(USER)
            WebChatRealtimeVoiceBackgroundProtocol.ACTION_HANG_UP -> requestHangUp()
            WebChatRealtimeVoiceBackgroundProtocol.ACTION_STOP -> stopSession()
        }
        return START_NOT_STICKY
    }

    override fun onDestroy() {
        overlay.hide()
        abandonAudioFocus()
        active = false
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun start(intent: Intent) {
        status = WebChatRealtimeVoiceBackgroundProtocol.status(
            intent.getStringExtra(WebChatRealtimeVoiceBackgroundProtocol.EXTRA_STATUS),
        )
        detail = intent.getStringExtra(WebChatRealtimeVoiceBackgroundProtocol.EXTRA_DETAIL)
            ?.take(MAX_DETAIL_LENGTH).orEmpty().ifBlank { "实时语音正在进行" }
        hostVisible = intent.getBooleanExtra(
            WebChatRealtimeVoiceBackgroundProtocol.EXTRA_HOST_VISIBLE,
            true,
        )
        active = true
        startForegroundCompat(notification())
        requestAudioFocus()
        render()
    }

    private fun update(intent: Intent) {
        status = WebChatRealtimeVoiceBackgroundProtocol.status(
            intent.getStringExtra(WebChatRealtimeVoiceBackgroundProtocol.EXTRA_STATUS),
        )
        detail = intent.getStringExtra(WebChatRealtimeVoiceBackgroundProtocol.EXTRA_DETAIL)
            ?.take(MAX_DETAIL_LENGTH).orEmpty().ifBlank { detail }
        paused = status == WebChatRealtimeVoiceBackgroundStatus.PAUSED
        render()
    }

    private fun requestPause(source: WebChatRealtimeVoiceBackgroundControlSource) {
        if (!active || paused) return
        if (source == USER) userPaused = true else mediaPaused = true
        paused = true
        status = WebChatRealtimeVoiceBackgroundStatus.PAUSED
        detail = if (source == MEDIA) "其他媒体正在播放，语音已自动暂停" else "实时语音已暂停"
        if (source == USER) abandonAudioFocus()
        sendControl(WebChatRealtimeVoiceBackgroundControl.PAUSE, source)
        render()
    }

    private fun requestResume(source: WebChatRealtimeVoiceBackgroundControlSource) {
        if (!active || !paused) return
        if (source == USER) userPaused = false
        mediaPaused = false
        paused = false
        status = WebChatRealtimeVoiceBackgroundStatus.LISTENING
        detail = "实时语音已继续"
        requestAudioFocus()
        sendControl(WebChatRealtimeVoiceBackgroundControl.RESUME, source)
        render()
    }

    private fun requestHangUp() {
        if (!active) return
        status = WebChatRealtimeVoiceBackgroundStatus.CONNECTING
        detail = "正在结束实时语音"
        sendControl(WebChatRealtimeVoiceBackgroundControl.HANG_UP, USER)
        render()
    }

    private fun onAudioFocusChanged(change: Int) {
        when (change) {
            AudioManager.AUDIOFOCUS_GAIN -> {
                if (mediaPaused && !userPaused) requestResume(MEDIA)
            }
            AudioManager.AUDIOFOCUS_LOSS,
            AudioManager.AUDIOFOCUS_LOSS_TRANSIENT,
            AudioManager.AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK -> {
                if (!paused) requestPause(MEDIA)
            }
        }
    }

    private fun sendControl(
        control: WebChatRealtimeVoiceBackgroundControl,
        source: WebChatRealtimeVoiceBackgroundControlSource,
    ) {
        sendBroadcast(
            Intent(WebChatRealtimeVoiceBackgroundProtocol.ACTION_CONTROL)
                .setPackage(packageName)
                .putExtra(WebChatRealtimeVoiceBackgroundProtocol.EXTRA_CONTROL, control.wireValue)
                .putExtra(WebChatRealtimeVoiceBackgroundProtocol.EXTRA_SOURCE, source.wireValue),
        )
    }

    private fun render() {
        if (!active) return
        overlay.update(status, detail)
        if (hostVisible) overlay.hide() else overlay.show()
        getSystemService(NotificationManager::class.java).notify(NOTIFICATION_ID, notification())
    }

    private fun stopSession() {
        overlay.hide()
        abandonAudioFocus()
        active = false
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    private fun requestAudioFocus() {
        audioManager.requestAudioFocus(audioFocusRequest)
    }

    private fun abandonAudioFocus() {
        audioManager.abandonAudioFocusRequest(audioFocusRequest)
    }

    private fun openApp() {
        startActivity(
            Intent(this, MainActivity::class.java).apply {
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP)
            },
        )
    }

    private fun startForegroundCompat(notification: Notification) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(
                NOTIFICATION_ID,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE,
            )
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
    }

    private fun notification(): Notification {
        val pauseAction = if (paused) {
            notificationAction(
                android.R.drawable.ic_media_play,
                "继续",
                WebChatRealtimeVoiceBackgroundProtocol.ACTION_RESUME,
                REQUEST_RESUME,
            )
        } else {
            notificationAction(
                android.R.drawable.ic_media_pause,
                "暂停",
                WebChatRealtimeVoiceBackgroundProtocol.ACTION_PAUSE,
                REQUEST_PAUSE,
            )
        }
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_input_voice)
            .setContentTitle("语音 AI")
            .setContentText(detail)
            .setContentIntent(openAppPendingIntent())
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setCategory(NotificationCompat.CATEGORY_CALL)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .addAction(pauseAction)
            .addAction(
                notificationAction(
                    R.drawable.ic_voice_call_hangup,
                    "挂断",
                    WebChatRealtimeVoiceBackgroundProtocol.ACTION_HANG_UP,
                    REQUEST_HANG_UP,
                ),
            )
            .build()
    }

    private fun notificationAction(icon: Int, title: String, action: String, requestCode: Int) =
        NotificationCompat.Action(icon, title, servicePendingIntent(action, requestCode))

    private fun servicePendingIntent(action: String, requestCode: Int): PendingIntent =
        PendingIntent.getService(
            this,
            requestCode,
            Intent(this, WebChatRealtimeVoiceBackgroundService::class.java).setAction(action),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

    private fun openAppPendingIntent(): PendingIntent = PendingIntent.getActivity(
        this,
        REQUEST_OPEN_APP,
        Intent(this, MainActivity::class.java).apply {
            addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP)
        },
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
    )

    private fun createNotificationChannel() {
        getSystemService(NotificationManager::class.java).createNotificationChannel(
            NotificationChannel(
                CHANNEL_ID,
                "语音 AI 后台会话",
                NotificationManager.IMPORTANCE_LOW,
            ).apply {
                description = "在其他应用中继续、暂停或挂断实时语音"
                setShowBadge(false)
            },
        )
    }

    private companion object {
        const val CHANNEL_ID = "web_chat_realtime_voice"
        const val NOTIFICATION_ID = 2028
        const val REQUEST_OPEN_APP = 0
        const val REQUEST_PAUSE = 1
        const val REQUEST_RESUME = 2
        const val REQUEST_HANG_UP = 3
        const val MAX_DETAIL_LENGTH = 96
        val USER = WebChatRealtimeVoiceBackgroundControlSource.USER
        val MEDIA = WebChatRealtimeVoiceBackgroundControlSource.MEDIA
    }
}
