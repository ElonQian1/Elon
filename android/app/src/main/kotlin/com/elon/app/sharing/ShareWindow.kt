package com.elon.app.sharing

import android.view.View
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import com.elon.app.R
import com.elon.app.elonColor

internal fun AppCompatActivity.installShareWindow(root: View) {
    root.setBackgroundColor(elonColor(R.color.elon_bg_app))
    WindowCompat.setDecorFitsSystemWindows(window, false)
    ViewCompat.setOnApplyWindowInsetsListener(root) { view, insets ->
        val safe = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.ime())
        view.setPadding(safe.left, safe.top, safe.right, safe.bottom); insets
    }
    setContentView(root)
}
