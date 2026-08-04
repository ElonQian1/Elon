package com.elon.app

import android.content.Context
import androidx.annotation.ColorInt
import androidx.annotation.ColorRes
import androidx.core.content.ContextCompat

@ColorInt
internal fun Context.elonColor(@ColorRes colorRes: Int): Int =
    ContextCompat.getColor(this, colorRes)
