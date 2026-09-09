package com.elon.app.grid.create

/** A prepared command is bound to one caller operation, account, document and exact draft. */
internal class BinanceCreatePermit(private val now: () -> Long) {
    private var operation = ""
    private var account = ""
    private var document = ""
    private var digest = ""
    private var nonce = ""
    private var expires = 0L
    fun bind(operation: String, account: String, document: String, digest: String, nonce: String) {
        require(listOf(operation, account, digest, nonce).all { Regex("[a-f0-9]{64}").matches(it) })
        require(document.isNotEmpty())
        this.operation = operation; this.account = account; this.document = document
        this.digest = digest; this.nonce = nonce; expires = now() + 55_000
    }
    fun valid(operation: String, account: String?, document: String, digest: String, nonce: String) =
        this.nonce.isNotEmpty() && now() < expires && this.operation == operation && this.account == account &&
            this.document == document && this.digest == digest && this.nonce == nonce
    fun consume(operation: String, account: String?, document: String, digest: String, nonce: String) {
        require(valid(operation, account, document, digest, nonce)) { "准备已变化或过期，请重新检查" }
        clear()
    }
    fun clear() { nonce = ""; expires = 0 }
}
