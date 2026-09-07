package com.elon.app.privateaccess

/** Bounded strict JSON for the private wire contract. No coercion or duplicate keys. */
internal object StrictJson {
    class Number(val text: String)

    fun parse(raw: String, maxBytes: Int = 262144): Map<String, Any?> {
        require(raw.toByteArray(Charsets.UTF_8).size <= maxBytes)
        val parser = Parser(raw)
        val result = parser.value(0)
        parser.space()
        require(parser.done())
        @Suppress("UNCHECKED_CAST")
        return result as? Map<String, Any?> ?: error("PRIVATE_JSON_INVALID")
    }

    fun encode(value: Any?): String = when (value) {
        null -> "null"
        is String -> buildString {
            append('"')
            value.forEach { c -> when (c) {
                '"' -> append("\\\""); '\\' -> append("\\\\")
                '\n' -> append("\\n"); '\r' -> append("\\r"); '\t' -> append("\\t")
                else -> if (c < ' ') append("\\u%04x".format(c.code)) else append(c)
            } }
            append('"')
        }
        is Boolean, is Int, is Long -> value.toString()
        is Number -> value.text
        is Map<*, *> -> value.entries.joinToString(",", "{", "}") { encode(it.key as String) + ":" + encode(it.value) }
        is List<*> -> value.joinToString(",", "[", "]") { encode(it) }
        else -> error("PRIVATE_JSON_INVALID")
    }

    private class Parser(val raw: String) {
        var at = 0
        var nodes = 0
        fun done() = at == raw.length
        fun space() { while (at < raw.length && raw[at] in " \r\n\t") at++ }
        fun take(c: Char): Boolean { space(); if (at < raw.length && raw[at] == c) { at++; return true }; return false }
        fun value(depth: Int): Any? {
            require(depth <= 14 && ++nodes <= 50000)
            space(); require(at < raw.length)
            return when (raw[at]) {
                '{' -> {
                    at++; val result = linkedMapOf<String, Any?>()
                    if (!take('}')) do {
                        space(); val key = string(); require(key.length <= 128 && !result.containsKey(key))
                        require(take(':')); result[key] = value(depth + 1)
                        if (take('}')) break
                        require(take(','))
                    } while (true)
                    result
                }
                '[' -> {
                    at++; val result = mutableListOf<Any?>()
                    if (!take(']')) do {
                        require(result.size < 500); result.add(value(depth + 1))
                        if (take(']')) break
                        require(take(','))
                    } while (true)
                    result
                }
                '"' -> string()
                't' -> literal("true", true)
                'f' -> literal("false", false)
                'n' -> literal("null", null)
                else -> {
                    val start = at
                    while (at < raw.length && raw[at] in "0123456789.eE+-") at++
                    val text = raw.substring(start, at)
                    require(Regex("-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?").matches(text))
                    Number(text)
                }
            }
        }
        private fun literal(text: String, value: Any?): Any? {
            require(raw.startsWith(text, at)); at += text.length; return value
        }
        private fun string(): String {
            require(at < raw.length && raw[at++] == '"')
            val result = StringBuilder()
            while (at < raw.length) {
                val c = raw[at++]
                if (c == '"') return result.toString().also { require(validUnicode(it)) }
                require(c >= ' ')
                if (c != '\\') result.append(c) else {
                    require(at < raw.length)
                    when (val escaped = raw[at++]) {
                        '"', '\\', '/' -> result.append(escaped)
                        'b' -> result.append('\b'); 'f' -> result.append('\u000c')
                        'n' -> result.append('\n'); 'r' -> result.append('\r'); 't' -> result.append('\t')
                        'u' -> {
                            require(at + 4 <= raw.length)
                            val hex = raw.substring(at, at + 4)
                            require(Regex("[0-9a-fA-F]{4}").matches(hex))
                            result.append(hex.toInt(16).toChar()); at += 4
                        }
                        else -> error("PRIVATE_JSON_INVALID")
                    }
                }
                require(result.length <= 4096)
            }
            error("PRIVATE_JSON_INVALID")
        }
        private fun validUnicode(value: String): Boolean {
            var index = 0
            while (index < value.length) {
                val c = value[index++]
                if (Character.isHighSurrogate(c)) {
                    if (index == value.length || !Character.isLowSurrogate(value[index++])) return false
                } else if (Character.isLowSurrogate(c)) return false
            }
            return true
        }
    }
}

internal fun Map<String, Any?>.exact(vararg keys: String) = require(this.keys == keys.toSet())
internal fun Map<String, Any?>.text(key: String): String = this[key] as? String ?: error("PRIVATE_FIELD_INVALID")
internal fun Map<String, Any?>.number(key: String): Long {
    val text = (this[key] as? StrictJson.Number)?.text ?: error("PRIVATE_FIELD_INVALID")
    require(Regex("0|[1-9][0-9]*").matches(text)); return text.toLong()
}
internal fun Map<String, Any?>.flag(key: String): Boolean = this[key] as? Boolean ?: error("PRIVATE_FIELD_INVALID")
@Suppress("UNCHECKED_CAST")
internal fun Map<String, Any?>.obj(key: String): Map<String, Any?> = this[key] as? Map<String, Any?> ?: error("PRIVATE_FIELD_INVALID")
internal fun Map<String, Any?>.items(key: String): List<*> = this[key] as? List<*> ?: error("PRIVATE_FIELD_INVALID")
