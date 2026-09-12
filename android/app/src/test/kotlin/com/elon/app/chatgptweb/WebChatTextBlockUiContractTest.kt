package com.elon.app.chatgptweb

import java.io.File
import org.junit.Assert.*
import org.junit.Test

class WebChatTextBlockUiContractTest {
    private fun editor() = File("src/main/kotlin/com/elon/app/WebChatTextBlockEditor.kt").readText()

    @Test fun readOnlyCloudPreparationDoesNotDisableLocalEditingOrClosing() {
        val source = editor()
        assertTrue(source.contains("val working = saving || (cloud?.busy == true && cloud.pending)"))
        assertTrue(source.contains("if (saving || (cloud?.busy == true && cloud.pending)) return"))
        assertTrue(source.contains("cloud != null && !cloud.busy"))
        assertTrue(source.contains("cloud?.prepare()"))
        assertTrue(source.contains("cloud?.close()"))
    }

    @Test fun exportConfirmationIsNotHiddenByAnUnavailableCloudTicketAndIconsHaveContrast() {
        val source = editor()
        val exported = source.indexOf("exported == body.text.toString() ->")
        val pending = source.indexOf("cloud?.pending == true ->")
        val preparing = source.indexOf("cloud?.busy == true || cloud != null && !cloud.ready ->")
        assertTrue(pending >= 0 && pending < exported && exported < preparing)
        assertTrue(source.contains("imageTintList = android.content.res.ColorStateList("))
        assertTrue(source.contains("intArrayOf(status.currentTextColor,"))
    }

    @Test fun verifiedCloudSaveTakesPrecedenceOverTheEarlierExportOfTheSameBody() {
        val source = editor()
        val pending = source.indexOf("cloud?.pending == true ->")
        val saved = source.indexOf("cloud?.savedToCloud == true && cloud.ready && !cloud.busy && !changed ->")
        val exported = source.indexOf("exported == body.text.toString() ->")
        assertTrue(pending >= 0 && pending < saved && saved < exported)
        val session = File("src/main/kotlin/com/elon/app/WebChatTextBlockCloudSession.kt").readText()
        assertTrue(session.contains("var savedToCloud = false; private set"))
        assertTrue(session.contains("receipt.detail == \"writing_saved\" && !pending"))
        assertTrue(session.contains("savedContent = content; savedToCloud = true; submitted = null"))
    }
}
