package com.elon.app

import android.content.res.ColorStateList
import android.widget.ImageView

/** Monochrome composer glyphs share the foreground of the themed input surface. */
internal fun ImageView.setComposerIcon(resourceId: Int) {
    setImageResource(resourceId)
    imageTintList = ColorStateList.valueOf(context.getColor(R.color.mobile_on_surface))
}
