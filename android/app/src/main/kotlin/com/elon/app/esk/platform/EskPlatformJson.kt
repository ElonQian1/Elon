package com.elon.app.esk.platform

import com.google.gson.stream.JsonReader
import com.google.gson.stream.JsonToken
import java.io.StringReader
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction

/** Shared strict JSON mechanics; each caller retains its own schema and fixed error boundary. */
internal object EskPlatformJson {
    const val MAX_BYTES = 64 * 1024
    private const val ERROR = "ESK_PLATFORM_ACCOUNT_INVALID"
    internal data class NumberToken(val raw: String)

    fun readObject(bytes: ByteArray): Map<String, Any?> {
        require(bytes.size in 1..MAX_BYTES)
        val text = Charsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(bytes)).toString()
        require(!text.startsWith('\uFEFF'))
        validateStringLexemes(text)
        val root = JsonReader(StringReader(text)).use { reader ->
            reader.isLenient = false
            val value = readValue(reader, 0, intArrayOf(0)).asObject()
            require(reader.peek() == JsonToken.END_DOCUMENT)
            value
        }
        return root
    }

    private fun readValue(reader: JsonReader, depth: Int, nodes: IntArray): Any? {
        require(depth <= 6 && ++nodes[0] <= 2048)
        return when (reader.peek()) {
            JsonToken.BEGIN_OBJECT -> {
                reader.beginObject()
                val result = linkedMapOf<String, Any?>()
                while (reader.hasNext()) {
                    require(result.size < 64)
                    val key = reader.nextName()
                    require(key.length <= 64 && validUtf16(key) && !result.containsKey(key))
                    result[key] = readValue(reader, depth + 1, nodes)
                }
                reader.endObject()
                result
            }
            JsonToken.BEGIN_ARRAY -> {
                reader.beginArray()
                val result = mutableListOf<Any?>()
                while (reader.hasNext()) {
                    require(result.size < 100)
                    result.add(readValue(reader, depth + 1, nodes))
                }
                reader.endArray()
                result.toList()
            }
            JsonToken.STRING -> reader.nextString().also { require(it.length <= 2048 && validUtf16(it)) }
            JsonToken.NUMBER -> NumberToken(reader.nextString().also { require(it.length <= 32) })
            JsonToken.BOOLEAN -> reader.nextBoolean()
            JsonToken.NULL -> { reader.nextNull(); null }
            else -> error(ERROR)
        }
    }

    // Gson versions differ on raw controls and non-JSON escapes inside otherwise quoted strings.
    private fun validateStringLexemes(text: String) {
        var quoted = false
        var index = 0
        while (index < text.length) {
            val char = text[index++]
            if (!quoted) {
                if (char == '"') quoted = true
            } else when {
                char == '"' -> quoted = false
                char == '\\' -> {
                    require(index < text.length)
                    val escaped = text[index++]
                    require(escaped in "\"\\/bfnrtu")
                    if (escaped == 'u') {
                        require(index + 4 <= text.length)
                        repeat(4) { require(text[index++] in "0123456789abcdefABCDEF") }
                    }
                }
                else -> require(char.code >= 0x20)
            }
        }
        require(!quoted)
    }

    private fun validUtf16(value: String): Boolean {
        var index = 0
        while (index < value.length) {
            val char = value[index++]
            if (Character.isHighSurrogate(char)) {
                if (index == value.length || !Character.isLowSurrogate(value[index++])) return false
            } else if (Character.isLowSurrogate(char)) return false
        }
        return true
    }

    @Suppress("UNCHECKED_CAST")
    private fun Any?.asObject(): Map<String, Any?> = this as? Map<String, Any?> ?: error(ERROR)
}
