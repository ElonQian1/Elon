// infrastructure/network/WebSocketServer.kt
// module: infrastructure/network | layer: infrastructure | role: websocket-server
// summary: WebSocket 双向通信服务器，支持 PC-手机协同

package com.elon.app.agent.infrastructure.network

import android.util.Log
import com.google.gson.Gson
import com.google.gson.JsonParser
import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.*
import java.io.*
import java.net.ServerSocket
import java.net.Socket
import java.security.MessageDigest
import java.util.*
import java.util.concurrent.ConcurrentHashMap

/**
 * WebSocket 服务器
 * 
 * 支持功能：
 * - PC → 手机：发送命令、目标
 * - 手机 → PC：上报状态、进度、屏幕
 * - 双向：实时同步
 */
class WebSocketServer(
    private val port: Int = 11452
) {
    companion object {
        private const val TAG = "WebSocketServer"
        private const val WEBSOCKET_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
    }
    
    private var serverSocket: ServerSocket? = null
    private val clients = ConcurrentHashMap<String, ClientConnection>()
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private val gson = Gson()
    
    // 消息处理器
    private val messageHandlers = mutableMapOf<String, MessageHandler>()
    
    // 事件流
    private val _events = MutableSharedFlow<ServerEvent>(replay = 1)
    val events: SharedFlow<ServerEvent> = _events.asSharedFlow()
    
    // 接收的消息队列
    private val incomingMessages = Channel<IncomingMessage>(Channel.BUFFERED)
    
    /**
     * 启动服务器
     */
    fun start() {
        scope.launch {
            try {
                serverSocket = ServerSocket(port)
                Log.i(TAG, "WebSocket 服务器启动在端口 $port")
                _events.emit(ServerEvent.Started(port))
                
                while (isActive) {
                    try {
                        val socket = serverSocket?.accept() ?: break
                        handleNewConnection(socket)
                    } catch (e: Exception) {
                        if (isActive) {
                            Log.e(TAG, "接受连接失败", e)
                        }
                    }
                }
            } catch (e: Exception) {
                Log.e(TAG, "服务器启动失败", e)
                _events.emit(ServerEvent.Error(e.message ?: "启动失败"))
            }
        }
    }
    
    /**
     * 停止服务器
     */
    fun stop() {
        Log.i(TAG, "停止 WebSocket 服务器")
        clients.values.forEach { it.close() }
        clients.clear()
        serverSocket?.close()
        serverSocket = null
        scope.cancel()
    }
    
    /**
     * 注册消息处理器
     */
    fun registerHandler(type: String, handler: MessageHandler) {
        messageHandlers[type] = handler
    }
    
    /**
     * 广播消息给所有客户端
     */
    suspend fun broadcast(message: OutgoingMessage) {
        val json = gson.toJson(message)
        clients.values.forEach { client ->
            try {
                client.send(json)
            } catch (e: Exception) {
                Log.w(TAG, "广播失败: ${client.id}", e)
            }
        }
    }
    
    /**
     * 发送消息给指定客户端
     */
    suspend fun sendTo(clientId: String, message: OutgoingMessage) {
        val json = gson.toJson(message)
        clients[clientId]?.send(json)
    }
    
    /**
     * 获取连接的客户端数量
     */
    fun getClientCount(): Int = clients.size
    
    /**
     * 处理新连接
     */
    private fun handleNewConnection(socket: Socket) {
        scope.launch {
            try {
                val reader = BufferedReader(InputStreamReader(socket.getInputStream()))
                val writer = BufferedOutputStream(socket.getOutputStream())
                
                // 处理 WebSocket 握手
                val handshakeResult = performHandshake(reader, writer)
                if (!handshakeResult.success) {
                    socket.close()
                    return@launch
                }
                
                val clientId = UUID.randomUUID().toString().take(8)
                val connection = ClientConnection(
                    id = clientId,
                    socket = socket,
                    reader = socket.getInputStream(),
                    writer = writer
                )
                
                clients[clientId] = connection
                Log.i(TAG, "客户端连接: $clientId (总数: ${clients.size})")
                _events.emit(ServerEvent.ClientConnected(clientId))
                
                // 发送欢迎消息
                connection.send(gson.toJson(OutgoingMessage(
                    type = "welcome",
                    payload = mapOf(
                        "clientId" to clientId,
                        "serverTime" to System.currentTimeMillis()
                    )
                )))
                
                // 开始读取消息
                readMessages(connection)
                
            } catch (e: Exception) {
                Log.e(TAG, "处理连接失败", e)
                socket.close()
            }
        }
    }
    
    /**
     * WebSocket 握手
     */
    private suspend fun performHandshake(
        reader: BufferedReader,
        writer: OutputStream
    ): HandshakeResult {
        // 读取 HTTP 请求头
        val headers = mutableMapOf<String, String>()
        var line = reader.readLine()
        
        // 第一行是请求行
        if (!line.startsWith("GET")) {
            return HandshakeResult(false, "Not a GET request")
        }
        
        // 读取其余头部
        line = reader.readLine()
        while (line != null && line.isNotEmpty()) {
            val colonIndex = line.indexOf(":")
            if (colonIndex > 0) {
                val key = line.substring(0, colonIndex).trim()
                val value = line.substring(colonIndex + 1).trim()
                headers[key] = value
            }
            line = reader.readLine()
        }
        
        // 验证 WebSocket 升级请求
        val upgrade = headers["Upgrade"]
        val secKey = headers["Sec-WebSocket-Key"]
        
        if (upgrade?.lowercase() != "websocket" || secKey == null) {
            return HandshakeResult(false, "Not a WebSocket upgrade request")
        }
        
        // 计算响应 Key
        val acceptKey = calculateAcceptKey(secKey)
        
        // 发送握手响应
        val response = """
            HTTP/1.1 101 Switching Protocols
            Upgrade: websocket
            Connection: Upgrade
            Sec-WebSocket-Accept: $acceptKey
            
        """.trimIndent().replace("\n", "\r\n") + "\r\n"
        
        writer.write(response.toByteArray())
        writer.flush()
        
        return HandshakeResult(true)
    }
    
    private fun calculateAcceptKey(key: String): String {
        val combined = key + WEBSOCKET_GUID
        val sha1 = MessageDigest.getInstance("SHA-1")
        val hash = sha1.digest(combined.toByteArray())
        return Base64.getEncoder().encodeToString(hash)
    }
    
    /**
     * 读取 WebSocket 消息
     */
    private suspend fun readMessages(connection: ClientConnection) {
        val input = connection.reader
        
        try {
            while (connection.isConnected) {
                val frame = readFrame(input) ?: break
                
                when (frame.opcode) {
                    0x01 -> { // Text frame
                        val message = String(frame.payload)
                        handleMessage(connection.id, message)
                    }
                    0x08 -> { // Close frame
                        Log.i(TAG, "客户端请求关闭: ${connection.id}")
                        break
                    }
                    0x09 -> { // Ping
                        // 发送 Pong
                        connection.sendRaw(createFrame(0x0A, frame.payload))
                    }
                    0x0A -> { // Pong
                        // 忽略
                    }
                }
            }
        } catch (e: Exception) {
            Log.w(TAG, "读取消息失败: ${connection.id}", e)
        } finally {
            clients.remove(connection.id)
            connection.close()
            Log.i(TAG, "客户端断开: ${connection.id} (剩余: ${clients.size})")
            _events.emit(ServerEvent.ClientDisconnected(connection.id))
        }
    }
    
    /**
     * 读取 WebSocket 帧
     */
    private fun readFrame(input: InputStream): WebSocketFrame? {
        val byte1 = input.read()
        if (byte1 == -1) return null
        
        val fin = (byte1 and 0x80) != 0
        val opcode = byte1 and 0x0F
        
        val byte2 = input.read()
        if (byte2 == -1) return null
        
        val masked = (byte2 and 0x80) != 0
        var payloadLength = (byte2 and 0x7F).toLong()
        
        // 读取扩展长度
        if (payloadLength == 126L) {
            val len1 = input.read()
            val len2 = input.read()
            payloadLength = ((len1 shl 8) or len2).toLong()
        } else if (payloadLength == 127L) {
            var len = 0L
            repeat(8) {
                len = (len shl 8) or input.read().toLong()
            }
            payloadLength = len
        }
        
        // 读取掩码
        val maskKey = if (masked) {
            ByteArray(4).also { input.read(it) }
        } else null
        
        // 读取载荷
        val payload = ByteArray(payloadLength.toInt())
        var read = 0
        while (read < payloadLength) {
            val r = input.read(payload, read, (payloadLength - read).toInt())
            if (r == -1) return null
            read += r
        }
        
        // 解码掩码
        if (masked && maskKey != null) {
            for (i in payload.indices) {
                payload[i] = (payload[i].toInt() xor maskKey[i % 4].toInt()).toByte()
            }
        }
        
        return WebSocketFrame(fin, opcode, payload)
    }
    
    /**
     * 创建 WebSocket 帧
     */
    private fun createFrame(opcode: Int, payload: ByteArray): ByteArray {
        val out = ByteArrayOutputStream()
        
        // 第一个字节：FIN + opcode
        out.write(0x80 or opcode)
        
        // 第二个字节：长度
        when {
            payload.size <= 125 -> {
                out.write(payload.size)
            }
            payload.size <= 65535 -> {
                out.write(126)
                out.write(payload.size shr 8)
                out.write(payload.size and 0xFF)
            }
            else -> {
                out.write(127)
                val len = payload.size.toLong()
                repeat(8) { i ->
                    out.write((len shr (56 - 8 * i)).toInt() and 0xFF)
                }
            }
        }
        
        out.write(payload)
        return out.toByteArray()
    }
    
    /**
     * 处理收到的消息
     */
    private suspend fun handleMessage(clientId: String, message: String) {
        try {
            val json = JsonParser.parseString(message).asJsonObject
            val type = json.get("type")?.asString ?: "unknown"
            val payload = json.get("payload")
            
            Log.d(TAG, "收到消息: $type from $clientId")
            
            // 查找处理器
            val handler = messageHandlers[type]
            if (handler != null) {
                val response = handler(clientId, payload?.toString() ?: "{}")
                if (response != null) {
                    sendTo(clientId, response)
                }
            } else {
                // 发送到消息队列
                incomingMessages.send(IncomingMessage(clientId, type, payload?.toString()))
            }
            
            _events.emit(ServerEvent.MessageReceived(clientId, type))
            
        } catch (e: Exception) {
            Log.e(TAG, "处理消息失败: $message", e)
        }
    }
    
    /**
     * 获取下一条消息（挂起）
     */
    suspend fun receiveMessage(): IncomingMessage {
        return incomingMessages.receive()
    }
    
    data class HandshakeResult(val success: Boolean, val error: String? = null)
    data class WebSocketFrame(val fin: Boolean, val opcode: Int, val payload: ByteArray)
}

/**
 * 客户端连接
 */
class ClientConnection(
    val id: String,
    private val socket: Socket,
    val reader: InputStream,
    private val writer: OutputStream
) {
    var isConnected = true
        private set
    
    private val lock = Any()
    
    suspend fun send(message: String) = withContext(Dispatchers.IO) {
        sendRaw(createTextFrame(message))
    }
    
    fun sendRaw(data: ByteArray) {
        synchronized(lock) {
            if (isConnected) {
                writer.write(data)
                writer.flush()
            }
        }
    }
    
    fun close() {
        isConnected = false
        try {
            socket.close()
        } catch (e: Exception) {
            // 忽略
        }
    }
    
    private fun createTextFrame(message: String): ByteArray {
        val payload = message.toByteArray()
        val out = ByteArrayOutputStream()
        
        out.write(0x81) // FIN + Text
        
        when {
            payload.size <= 125 -> out.write(payload.size)
            payload.size <= 65535 -> {
                out.write(126)
                out.write(payload.size shr 8)
                out.write(payload.size and 0xFF)
            }
            else -> {
                out.write(127)
                val len = payload.size.toLong()
                repeat(8) { i ->
                    out.write((len shr (56 - 8 * i)).toInt() and 0xFF)
                }
            }
        }
        
        out.write(payload)
        return out.toByteArray()
    }
}

/**
 * 消息处理器类型别名（支持 suspend lambda）
 */
typealias MessageHandler = suspend (clientId: String, payload: String) -> OutgoingMessage?

/**
 * 服务器事件
 */
sealed class ServerEvent {
    data class Started(val port: Int) : ServerEvent()
    data class Error(val message: String) : ServerEvent()
    data class ClientConnected(val clientId: String) : ServerEvent()
    data class ClientDisconnected(val clientId: String) : ServerEvent()
    data class MessageReceived(val clientId: String, val type: String) : ServerEvent()
}

/**
 * 收到的消息
 */
data class IncomingMessage(
    val clientId: String,
    val type: String,
    val payload: String?
)

/**
 * 发送的消息
 */
data class OutgoingMessage(
    val type: String,
    val payload: Any?,
    val timestamp: Long = System.currentTimeMillis()
)
