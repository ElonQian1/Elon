package com.elon.app

import android.text.InputType
import android.widget.EditText
import androidx.appcompat.app.AppCompatActivity

internal fun mainTitleEditText(
    activity: AppCompatActivity,
    value: String,
    dp: (Int) -> Int
): EditText {
    return EditText(activity).apply {
        setText(value)
        inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
        maxLines = 1
        setSingleLine(true)
        setSelectAllOnFocus(true)
        setPadding(dp(18), dp(8), dp(18), dp(8))
    }
}
