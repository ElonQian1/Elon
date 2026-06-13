// ui/SettingsActivity.kt
// module: ui | layer: presentation | role: 设置页面
// summary: 用户设置、账号管理 - 支持未登录/已登录两种状态

package com.elon.app.agent.ui

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.os.Bundle
import android.view.Gravity
import android.view.View
import android.widget.*
import com.elon.app.agent.AgentConfigActivity
import com.elon.app.agent.infrastructure.auth.AuthService

private const val SETTINGS_BG = "#101010"
private const val SETTINGS_CARD = "#222222"
private const val SETTINGS_BORDER = "#2E2E2E"
private const val SETTINGS_TEXT_PRIMARY = "#D6D6D6"
private const val SETTINGS_TEXT_SECONDARY = "#A8A8A8"
private const val SETTINGS_TEXT_TERTIARY = "#777777"
private const val SETTINGS_PRIMARY_BG = "#C8C8C8"
private const val SETTINGS_PRIMARY_TEXT = "#101010"
private const val SETTINGS_SECONDARY_BG = "#2A2A2A"
private const val SETTINGS_SECONDARY_TEXT = "#D6D6D6"
private const val SETTINGS_LINK = "#58BE6A"
private const val SETTINGS_DANGER = "#D97A7A"

/**
 * 设置页面
 * - 未登录：显示"登录/注册"按钮，引导用户开启云端同步
 * - 已登录：显示账号信息和登出选项
 */
class SettingsActivity : Activity() {
    
    private lateinit var authService: AuthService
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        authService = AuthService(this)
        setContentView(createLayout())
    }
    
    override fun onResume() {
        super.onResume()
        // 从登录页面返回后刷新界面
        setContentView(createLayout())
    }
    
    private fun createLayout(): View {
        val isLoggedIn = authService.isLoggedIn()
        
        return ScrollView(this).apply {
            setBackgroundColor(Color.parseColor(SETTINGS_BG))

            addView(LinearLayout(context).apply {
                orientation = LinearLayout.VERTICAL
                setBackgroundColor(Color.parseColor(SETTINGS_BG))
                
                // 标题栏
                addView(createHeader())
                
                // 账号卡片（根据登录状态显示不同内容）
                addView(if (isLoggedIn) createLoggedInCard() else createNotLoggedInCard())
                
                // 功能设置卡片
                addView(createFunctionCard())
                
                // 版本信息
                addView(createVersionInfo())
            })
        }
    }
    
    private fun createHeader(): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setBackgroundColor(Color.parseColor(SETTINGS_CARD))
            setPadding(32, 32, 32, 32)
            elevation = 4f

            addView(Button(context).apply {
                text = "← 返回"
                setBackgroundColor(Color.TRANSPARENT)
                setTextColor(Color.parseColor(SETTINGS_LINK))
                setOnClickListener { finish() }
            })

            addView(TextView(context).apply {
                text = "设置"
                textSize = 18f
                setTypeface(null, Typeface.BOLD)
                setTextColor(Color.parseColor(SETTINGS_TEXT_PRIMARY))
                gravity = Gravity.CENTER
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            })

            addView(View(context).apply {
                layoutParams = LinearLayout.LayoutParams(120, 1)
            })
        }
    }
    
    /**
     * 未登录状态：显示登录引导
     */
    private fun createNotLoggedInCard(): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(SETTINGS_CARD))
            setPadding(32, 32, 32, 32)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = 16 }

            addView(TextView(context).apply {
                text = "☁️"
                textSize = 48f
                gravity = Gravity.CENTER
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { bottomMargin = 16 }
            })

            addView(TextView(context).apply {
                text = "开启云端同步"
                textSize = 18f
                setTypeface(null, Typeface.BOLD)
                setTextColor(Color.parseColor(SETTINGS_TEXT_PRIMARY))
                gravity = Gravity.CENTER
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { bottomMargin = 8 }
            })

            addView(TextView(context).apply {
                text = "登录后可以：\n• 数据云端备份，换机不丢失\n• 多设备同步任务和配置\n• 查看采集的评论线索"
                textSize = 14f
                setTextColor(Color.parseColor(SETTINGS_TEXT_SECONDARY))
                gravity = Gravity.CENTER
                setLineSpacing(8f, 1f)
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { bottomMargin = 24 }
            })

            addView(Button(context).apply {
                text = "登录 / 注册"
                textSize = 16f
                setBackgroundColor(Color.parseColor(SETTINGS_PRIMARY_BG))
                setTextColor(Color.parseColor(SETTINGS_PRIMARY_TEXT))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT, 140
                )
                setOnClickListener {
                    startActivity(Intent(context, LoginActivity::class.java))
                }
            })

            addView(TextView(context).apply {
                text = "暂不登录也可正常使用本地功能"
                textSize = 12f
                setTextColor(Color.parseColor(SETTINGS_TEXT_TERTIARY))
                gravity = Gravity.CENTER
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { topMargin = 16 }
            })
        }
    }
    
    /**
     * 已登录状态：显示账号信息和操作
     */
    private fun createLoggedInCard(): LinearLayout {
        val user = authService.getCurrentUser()
        
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(SETTINGS_CARD))
            setPadding(32, 32, 32, 32)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = 16 }

            // 头像和信息
            addView(LinearLayout(context).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { bottomMargin = 24 }

                addView(TextView(context).apply {
                    text = "👤"
                    textSize = 40f
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply { rightMargin = 24 }
                })

                addView(LinearLayout(context).apply {
                    orientation = LinearLayout.VERTICAL
                    layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)

                    addView(TextView(context).apply {
                        text = user?.nickname ?: user?.username ?: "用户"
                        textSize = 18f
                        setTypeface(null, Typeface.BOLD)
                        setTextColor(Color.parseColor(SETTINGS_TEXT_PRIMARY))
                    })
                    addView(TextView(context).apply {
                        text = "@${user?.username ?: ""}"
                        textSize = 14f
                        setTextColor(Color.parseColor(SETTINGS_TEXT_TERTIARY))
                    })
                })

                addView(TextView(context).apply {
                    text = "☁️ 已同步"
                    textSize = 12f
                    setTextColor(Color.parseColor(SETTINGS_LINK))
                })
            })

            // 分隔线
            addView(View(context).apply {
                setBackgroundColor(Color.parseColor(SETTINGS_BORDER))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT, 1
                ).apply { bottomMargin = 24 }
            })

            // 切换账号按钮
            addView(Button(context).apply {
                text = "切换账号"
                textSize = 16f
                setBackgroundColor(Color.parseColor(SETTINGS_SECONDARY_BG))
                setTextColor(Color.parseColor(SETTINGS_SECONDARY_TEXT))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT, 120
                ).apply { bottomMargin = 12 }
                setOnClickListener { showSwitchAccountDialog() }
            })

            // 退出登录按钮
            addView(Button(context).apply {
                text = "退出登录"
                textSize = 16f
                setBackgroundColor(Color.parseColor(SETTINGS_SECONDARY_BG))
                setTextColor(Color.parseColor(SETTINGS_DANGER))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT, 120
                )
                setOnClickListener { showLogoutDialog() }
            })
        }
    }
    
    private fun createFunctionCard(): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(SETTINGS_CARD))
            setPadding(32, 32, 32, 32)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = 16 }

            addView(TextView(context).apply {
                text = "功能设置"
                textSize = 14f
                setTextColor(Color.parseColor(SETTINGS_TEXT_TERTIARY))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { bottomMargin = 16 }
            })

            addView(Button(context).apply {
                text = "🤖 AI 配置"
                textSize = 16f
                setBackgroundColor(Color.parseColor(SETTINGS_PRIMARY_BG))
                setTextColor(Color.parseColor(SETTINGS_PRIMARY_TEXT))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT, 120
                )
                setOnClickListener {
                    startActivity(Intent(context, AgentConfigActivity::class.java))
                }
            })

            addView(Button(context).apply {
                text = "🖥️ 我的节点"
                textSize = 16f
                setBackgroundColor(Color.parseColor(SETTINGS_SECONDARY_BG))
                setTextColor(Color.parseColor(SETTINGS_SECONDARY_TEXT))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT, 120
                ).apply { topMargin = 12 }
                setOnClickListener {
                    startActivity(Intent(context, NodeActivity::class.java))
                }
            })

            addView(Button(context).apply {
                text = "🏪 项目广场"
                textSize = 16f
                setBackgroundColor(Color.parseColor("#2A2A2A"))
                setTextColor(Color.parseColor("#D6D6D6"))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT, 120
                ).apply { topMargin = 12 }
                setOnClickListener {
                    startActivity(Intent(context, ProjectPlazaActivity::class.java))
                }
            })
        }
    }
    
    private fun createVersionInfo(): TextView {
        return TextView(this).apply {
            text = "营销助手 v1.0.0"
            textSize = 12f
            setTextColor(Color.parseColor(SETTINGS_TEXT_TERTIARY))
            gravity = Gravity.CENTER
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = 48 }
        }
    }
    
    private fun showSwitchAccountDialog() {
        AlertDialog.Builder(this)
            .setTitle("切换账号")
            .setMessage("确定要切换到其他账号吗？")
            .setPositiveButton("确定") { _, _ ->
                authService.logout()
                startActivity(Intent(this, LoginActivity::class.java))
            }
            .setNegativeButton("取消", null)
            .show()
    }
    
    private fun showLogoutDialog() {
        AlertDialog.Builder(this)
            .setTitle("退出登录")
            .setMessage("退出后数据将不再云端同步，确定退出吗？")
            .setPositiveButton("确定") { _, _ ->
                authService.logout()
                Toast.makeText(this, "已退出登录", Toast.LENGTH_SHORT).show()
                setContentView(createLayout()) // 刷新界面
            }
            .setNegativeButton("取消", null)
            .show()
    }
}
