// infrastructure/NotificationActionReceiver.kt
package com.elon.app.agent.infrastructure

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log
import androidx.localbroadcastmanager.content.LocalBroadcastManager
import com.elon.app.agent.AgentExecuteActivity
import com.elon.app.agent.infrastructure.floating.ConversationalVoiceActivity
import com.elon.app.agent.infrastructure.floating.FloatingInputActivity

/**
 * 📬 通知栏快捷操作接收器
 * 
 * 处理通知栏按钮点击事件
 */
class NotificationActionReceiver : BroadcastReceiver() {
    
    companion object {
        private const val TAG = "NotificationAction"
        
        const val ACTION_OPEN_APP = "com.elon.app.agent.ACTION_OPEN_APP"
        const val ACTION_QUICK_TASK = "com.elon.app.agent.ACTION_QUICK_TASK"
        const val ACTION_STOP = "com.elon.app.agent.ACTION_STOP"
        // 🆕 语音/文字输入动作
        const val ACTION_VOICE_INPUT = "com.elon.app.agent.ACTION_VOICE_INPUT"
        const val ACTION_TEXT_INPUT = "com.elon.app.agent.ACTION_TEXT_INPUT"
        
        // 预设任务
        const val TASK_OPEN_XHS = "打开小红书"
        const val TASK_HOT_NOTES = "打开小红书，找到点赞过万的热门笔记"
        const val TASK_CUSTOM = "custom"
    }
    
    override fun onReceive(context: Context?, intent: Intent?) {
        if (context == null || intent == null) return
        
        Log.i(TAG, "收到通知操作: ${intent.action}")
        
        when (intent.action) {
            ACTION_OPEN_APP -> {
                // 打开执行界面
                AgentExecuteActivity.start(context)
            }
            
            // 🆕 语音输入（智能对话系统 V2）
            ACTION_VOICE_INPUT -> {
                Log.i(TAG, "🎤 打开智能语音对话")
                val voiceIntent = Intent(context, ConversationalVoiceActivity::class.java).apply {
                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                    addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP)
                }
                context.startActivity(voiceIntent)
            }
            
            // 🆕 文字输入
            ACTION_TEXT_INPUT -> {
                Log.i(TAG, "⌨️ 打开文字输入")
                val textIntent = Intent(context, FloatingInputActivity::class.java).apply {
                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                    addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP)
                }
                context.startActivity(textIntent)
            }
            
            ACTION_QUICK_TASK -> {
                val task = intent.getStringExtra("task") ?: return
                Log.i(TAG, "执行快捷任务: $task")
                
                when (task) {
                    TASK_CUSTOM -> {
                        // 打开界面让用户输入
                        AgentExecuteActivity.start(context)
                    }
                    else -> {
                        // 直接执行预设任务
                        AgentExecuteActivity.start(context, goal = task, autoExecute = true)
                    }
                }
            }
            
            ACTION_STOP -> {
                // 发送停止广播
                LocalBroadcastManager.getInstance(context).sendBroadcast(
                    Intent("agent.stop")
                )
                Log.i(TAG, "已发送停止命令")
            }
        }
    }
}
