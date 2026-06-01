// ui/LoginActivity.kt
// module: ui | layer: presentation | role: 登录界面
// summary: 用户登录/注册入口（程序化布局）

package com.elon.app.agent.ui

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.text.InputType
import android.util.Log
import android.view.Gravity
import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.*
import com.elon.app.agent.infrastructure.auth.AuthService
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

private const val LOGIN_BG = "#101010"
private const val LOGIN_CARD = "#181B20"
private const val LOGIN_TEXT_PRIMARY = "#F2F5FA"
private const val LOGIN_TEXT_SECONDARY = "#A6AFBD"
private const val LOGIN_TEXT_TERTIARY = "#6F7785"
private const val LOGIN_PRIMARY_BG = "#58BE6A"
private const val LOGIN_PRIMARY_TEXT = "#07120A"
private const val LOGIN_LINK = "#6091CF"
private const val LOGIN_ERROR = "#D97A7A"

/**
 * 登录/注册界面（使用程序化布局）
 */
class LoginActivity : Activity() {
    
    companion object {
        private const val TAG = "LoginActivity"
    }
    
    private lateinit var authService: AuthService
    private val mainHandler = Handler(Looper.getMainLooper())
    private val scope = CoroutineScope(Dispatchers.Main)
    
    // UI 组件
    private lateinit var etUsername: EditText
    private lateinit var etPassword: EditText
    private lateinit var etNickname: EditText
    private lateinit var btnLogin: Button
    private lateinit var btnRegister: Button
    private lateinit var tvSwitchMode: TextView
    private lateinit var progressBar: ProgressBar
    private lateinit var tvError: TextView
    private lateinit var nicknameLayout: LinearLayout
    
    private var isRegisterMode = false
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        authService = AuthService(this)
        
        // 如果已登录，直接关闭（用户可能误点）
        if (authService.isLoggedIn()) {
            Toast.makeText(this, "您已登录", Toast.LENGTH_SHORT).show()
            finish()
            return
        }
        
        setContentView(createLayout())
        setupListeners()
    }
    
    private fun createLoadingLayout(): View {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setBackgroundColor(Color.parseColor(LOGIN_BG))
            
            addView(ProgressBar(context).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                )
            })
            
            addView(TextView(context).apply {
                text = "验证登录状态..."
                setTextColor(Color.parseColor(LOGIN_TEXT_SECONDARY))
                gravity = Gravity.CENTER
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { topMargin = 16 }
            })
        }
    }
    
    private fun createLayout(): View {
        return ScrollView(this).apply {
            setBackgroundColor(Color.parseColor(LOGIN_BG))
            
            addView(LinearLayout(context).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER_HORIZONTAL
                setPadding(64, 100, 64, 64)
                
                // 标题
                addView(TextView(context).apply {
                    text = "营销助手"
                    textSize = 28f
                    setTypeface(null, Typeface.BOLD)
                    setTextColor(Color.parseColor(LOGIN_TEXT_PRIMARY))
                })
                
                addView(TextView(context).apply {
                    text = "小红书智能运营工具"
                    textSize = 14f
                    setTextColor(Color.parseColor(LOGIN_TEXT_SECONDARY))
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply { bottomMargin = 80 }
                })
                
                // 用户名输入
                etUsername = EditText(context).apply {
                    hint = "用户名"
                    inputType = InputType.TYPE_CLASS_TEXT
                    setSingleLine(true)
                    imeOptions = EditorInfo.IME_ACTION_NEXT
                    setTextColor(Color.parseColor(LOGIN_TEXT_PRIMARY))
                    setHintTextColor(Color.parseColor(LOGIN_TEXT_TERTIARY))
                    setBackgroundColor(Color.parseColor(LOGIN_CARD))
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply { bottomMargin = 24 }
                }
                addView(etUsername)
                
                // 密码输入
                etPassword = EditText(context).apply {
                    hint = "密码"
                    inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
                    setSingleLine(true)
                    imeOptions = EditorInfo.IME_ACTION_DONE
                    setTextColor(Color.parseColor(LOGIN_TEXT_PRIMARY))
                    setHintTextColor(Color.parseColor(LOGIN_TEXT_TERTIARY))
                    setBackgroundColor(Color.parseColor(LOGIN_CARD))
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply { bottomMargin = 24 }
                }
                addView(etPassword)
                
                // 昵称输入（注册模式显示）
                nicknameLayout = LinearLayout(context).apply {
                    orientation = LinearLayout.VERTICAL
                    visibility = View.GONE
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply { bottomMargin = 24 }
                    
                    etNickname = EditText(context).apply {
                        hint = "昵称（选填）"
                        inputType = InputType.TYPE_CLASS_TEXT
                        setSingleLine(true)
                        setTextColor(Color.parseColor(LOGIN_TEXT_PRIMARY))
                        setHintTextColor(Color.parseColor(LOGIN_TEXT_TERTIARY))
                        setBackgroundColor(Color.parseColor(LOGIN_CARD))
                    }
                    addView(etNickname)
                }
                addView(nicknameLayout)
                
                // 错误提示
                tvError = TextView(context).apply {
                    setTextColor(Color.parseColor(LOGIN_ERROR))
                    textSize = 14f
                    visibility = View.GONE
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply { bottomMargin = 16 }
                }
                addView(tvError)
                
                // 登录按钮
                btnLogin = Button(context).apply {
                    text = "登 录"
                    textSize = 16f
                    setBackgroundColor(Color.parseColor(LOGIN_PRIMARY_BG))
                    setTextColor(Color.parseColor(LOGIN_PRIMARY_TEXT))
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        160
                    ).apply { bottomMargin = 16 }
                }
                addView(btnLogin)
                
                // 注册按钮
                btnRegister = Button(context).apply {
                    text = "注 册"
                    textSize = 16f
                    setBackgroundColor(Color.parseColor(LOGIN_PRIMARY_BG))
                    setTextColor(Color.parseColor(LOGIN_PRIMARY_TEXT))
                    visibility = View.GONE
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        160
                    ).apply { bottomMargin = 16 }
                }
                addView(btnRegister)
                
                // 加载指示器
                progressBar = ProgressBar(context).apply {
                    visibility = View.GONE
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply { topMargin = 16 }
                }
                addView(progressBar)
                
                // 切换模式
                tvSwitchMode = TextView(context).apply {
                    text = "没有账号？点击注册"
                    textSize = 14f
                    setTextColor(Color.parseColor(LOGIN_LINK))
                    setPadding(16, 32, 16, 16)
                }
                addView(tvSwitchMode)
            })
        }
    }
    
    private fun setupListeners() {
        btnLogin.setOnClickListener { performLogin() }
        btnRegister.setOnClickListener { performRegister() }
        
        tvSwitchMode.setOnClickListener {
            isRegisterMode = !isRegisterMode
            updateUI()
        }
    }
    
    private fun updateUI() {
        if (isRegisterMode) {
            nicknameLayout.visibility = View.VISIBLE
            btnLogin.visibility = View.GONE
            btnRegister.visibility = View.VISIBLE
            tvSwitchMode.text = "已有账号？点击登录"
        } else {
            nicknameLayout.visibility = View.GONE
            btnLogin.visibility = View.VISIBLE
            btnRegister.visibility = View.GONE
            tvSwitchMode.text = "没有账号？点击注册"
        }
        tvError.visibility = View.GONE
    }
    
    private fun performLogin() {
        val username = etUsername.text.toString().trim()
        val password = etPassword.text.toString()
        
        if (username.isEmpty() || password.isEmpty()) {
            showError("请输入用户名和密码")
            return
        }
        
        setLoading(true)
        
        scope.launch {
            try {
                val result = authService.login(username, password)
                mainHandler.post {
                    setLoading(false)
                    if (result.success) {
                        goToMain()
                    } else {
                        showError(result.message ?: "登录失败")
                    }
                }
            } catch (e: Exception) {
                Log.e(TAG, "Login error", e)
                mainHandler.post {
                    setLoading(false)
                    showError("网络错误: ${e.message}")
                }
            }
        }
    }
    
    private fun performRegister() {
        val username = etUsername.text.toString().trim()
        val password = etPassword.text.toString()
        val nickname = etNickname.text.toString().trim().takeIf { it.isNotEmpty() }
        
        if (username.isEmpty() || password.isEmpty()) {
            showError("请输入用户名和密码")
            return
        }
        
        if (username.length < 3) {
            showError("用户名至少3个字符")
            return
        }
        
        if (password.length < 6) {
            showError("密码至少6个字符")
            return
        }
        
        setLoading(true)
        
        scope.launch {
            try {
                val result = authService.register(username, password, nickname)
                mainHandler.post {
                    setLoading(false)
                    if (result.success) {
                        goToMain()
                    } else {
                        showError(result.message ?: "注册失败")
                    }
                }
            } catch (e: Exception) {
                Log.e(TAG, "Register error", e)
                mainHandler.post {
                    setLoading(false)
                    showError("网络错误: ${e.message}")
                }
            }
        }
    }
    
    private fun verifyAndProceed() {
        scope.launch {
            try {
                val result = authService.verifyToken()
                mainHandler.post {
                    if (result.success) {
                        goToMain()
                    } else {
                        // Token 无效，显示登录界面
                        setContentView(createLayout())
                        setupListeners()
                    }
                }
            } catch (e: Exception) {
                Log.e(TAG, "Token verification error", e)
                mainHandler.post {
                    setContentView(createLayout())
                    setupListeners()
                }
            }
        }
    }
    
    private fun goToMain() {
        // 登录成功，直接关闭返回设置页（设置页会刷新显示已登录状态）
        Toast.makeText(this, "登录成功 ✓", Toast.LENGTH_SHORT).show()
        finish()
    }
    
    private fun showError(message: String) {
        tvError.text = message
        tvError.visibility = View.VISIBLE
    }
    
    private fun setLoading(loading: Boolean) {
        progressBar.visibility = if (loading) View.VISIBLE else View.GONE
        btnLogin.isEnabled = !loading
        btnRegister.isEnabled = !loading
        etUsername.isEnabled = !loading
        etPassword.isEnabled = !loading
        etNickname.isEnabled = !loading
    }
}
