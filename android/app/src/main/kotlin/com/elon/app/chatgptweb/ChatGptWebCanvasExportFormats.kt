package com.elon.app.chatgptweb

internal data class ChatGptWebCanvasExportFormat(val key: String, val extension: String, val mediaType: String, val label: String)

internal object ChatGptWebCanvasExportFormats {
    private val documents = listOf(
        ChatGptWebCanvasExportFormat("md", "md", "text/markdown", "Markdown (.md)"),
        ChatGptWebCanvasExportFormat("pdf", "pdf", "application/pdf", "PDF"),
        ChatGptWebCanvasExportFormat("docx", "docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document", "Word (.docx)"),
    )
    // The reviewed official export table. Unknown code types stay unsupported.
    private val source = mapOf(
        "bash" to ("sh" to "text/x-shellscript"), "zsh" to ("zsh" to "text/x-shellscript"),
        "javascript" to ("js" to "application/javascript"), "typescript" to ("ts" to "application/typescript"),
        "html" to ("html" to "text/html"), "css" to ("css" to "text/css"), "python" to ("py" to "text/x-python"),
        "json" to ("json" to "application/json"), "sql" to ("sql" to "application/sql"), "go" to ("go" to "text/x-go"),
        "yaml" to ("yaml" to "application/x-yaml"), "java" to ("java" to "text/x-java-source"), "rust" to ("rs" to "text/rust"),
        "cpp" to ("cpp" to "text/x-c++src"), "swift" to ("swift" to "text/swift"), "php" to ("php" to "application/x-httpd-php"),
        "xml" to ("xml" to "application/xml"), "ruby" to ("rb" to "text/x-ruby"), "haskell" to ("hs" to "text/x-haskell"),
        "kotlin" to ("kt" to "text/x-kotlin"), "csharp" to ("cs" to "text/x-csharp"), "c" to ("c" to "text/x-csrc"),
        "objectivec" to ("m" to "text/x-objectivec"), "r" to ("r" to "text/plain"), "lua" to ("lua" to "text/x-lua"),
        "dart" to ("dart" to "text/x-dart"), "scala" to ("scala" to "text/x-scala"), "perl" to ("pl" to "text/x-perl"),
        "commonlisp" to ("lisp" to "text/x-common-lisp"), "clojure" to ("clj" to "text/x-clojure"), "ocaml" to ("ml" to "text/x-ocaml"),
        "powershell" to ("ps1" to "text/x-powershell"), "verilog" to ("v" to "text/x-verilog"),
        "dockerfile" to ("Dockerfile" to "text/x-dockerfile"), "vue" to ("vue" to "text/x-vue"),
        "react" to ("jsx" to "application/javascript"), "other" to ("txt" to "text/plain"),
    ).mapKeys { "code/${it.key}" }.mapValues { (_, value) ->
        ChatGptWebCanvasExportFormat("source", value.first, value.second, "源文件 (.${value.first})")
    }

    fun options(documentType: String): List<ChatGptWebCanvasExportFormat> =
        if (documentType == "document") documents else listOfNotNull(source[documentType])

    fun find(documentType: String, format: String): ChatGptWebCanvasExportFormat? =
        options(documentType).singleOrNull { it.key == format }
}
