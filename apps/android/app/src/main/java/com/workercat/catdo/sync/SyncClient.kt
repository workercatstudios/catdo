package com.workercat.catdo.sync

import com.clerk.api.Clerk
import com.clerk.api.network.serialization.ClerkResult
import com.clerk.api.network.serialization.errorMessage
import com.workercat.catdo.data.CatDoRepository
import com.workercat.catdo.data.DataJson
import com.workercat.catdo.data.Snapshot
import java.net.HttpURLConnection
import java.net.URI
import java.net.URL
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.currentCoroutineContext
import org.json.JSONObject
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response as SocketResponse
import okhttp3.WebSocket
import okhttp3.WebSocketListener

private const val API = "https://catdo.workercat.com"
const val TERMS_REVIEW_URL = "$API/app"

private data class Response(val status: Int, val json: JSONObject)

class SyncHttpException(val status: Int, val endpoint: String) : IllegalStateException(
    when (status) {
        401 -> "Sign-in was rejected by CatDo. Your tasks are safe on this device."
        428 -> "Review WorkerCat terms and confirm you are 13+ in your browser using this CatDo account. Then retry sync. Your tasks are saved here."
        else -> "Sync is unavailable ($status). Your tasks are safe on this device."
    }
) {
    val requiresTerms: Boolean get() = status == 428
}

private fun request(method: String, address: String, bearer: String, body: String? = null): Response {
    val uri = URI(address)
    require(uri.scheme == "https" && uri.host == URI(API).host) { "Unexpected sync server." }
    val connection = (URL(address).openConnection() as HttpURLConnection).apply {
        requestMethod = method
        connectTimeout = 20_000
        readTimeout = 20_000
        instanceFollowRedirects = false
        setRequestProperty("Accept", "application/json")
        setRequestProperty("Authorization", "Bearer $bearer")
        if (body != null) {
            doOutput = true
            setRequestProperty("Content-Type", "application/json")
            outputStream.use { it.write(body.toByteArray(Charsets.UTF_8)) }
        }
    }
    return try {
        val status = connection.responseCode
        val input = if (status in 200..299) connection.inputStream else connection.errorStream
        Response(status, JSONObject(input?.bufferedReader()?.use { it.readText() } ?: "{}"))
    } finally { connection.disconnect() }
}

class SyncClient(private val repository: CatDoRepository) {
    private val mutex = Mutex()
    private val socketClient = OkHttpClient.Builder()
        .followRedirects(false)
        .pingInterval(25, java.util.concurrent.TimeUnit.SECONDS)
        .build()

    suspend fun awaitClerk() {
        val (ready, error) = combine(Clerk.isInitialized, Clerk.initializationError) { initialized, failure ->
            initialized to failure
        }.first { (initialized, failure) -> initialized || failure != null }
        if (!ready) throw IllegalStateException(
            if (error is java.net.UnknownHostException) "Can't reach sign-in. Check your connection or Private DNS and try again."
            else "Can't connect to sign-in. Check your connection and try again.", error
        )
    }

    private suspend fun token(): String {
        awaitClerk()
        return when (val result = Clerk.auth.getToken()) {
            is ClerkResult.Success -> result.value
            is ClerkResult.Failure -> error(result.errorMessage.ifBlank { "Sign in again to sync." })
        }
    }

    private fun identity(accessToken: String): String {
        val response = request("GET", "$API/api/me", accessToken)
        if (response.status !in 200..299) throw SyncHttpException(response.status, "identity")
        return response.json.getString("userId")
    }

    suspend fun sync(): Boolean = mutex.withLock {
        val access = token()
        withContext(Dispatchers.IO) {
            val owner = identity(access)
            repository.bindOwner(owner)
            repeat(3) {
                val state = repository.state.value
                require(state.conflict == null) { "Resolve the sync conflict first." }
                val shouldPush = state.pending != null || state.data != state.base.data
                val pending = if (shouldPush) repository.freezePending() else null
                val response = if (pending == null) request("GET", "$API/api/sync", access)
                else request("POST", "$API/api/sync", access, JSONObject().apply {
                    put("id", pending.id)
                    put("base", DataJson.encode(pending.base))
                    put("data", DataJson.encode(pending.data))
                }.toString())
                if (response.status !in 200..299 && response.status != 409)
                    throw SyncHttpException(response.status, "sync")
                val remote = Snapshot(response.json.getLong("revision"), DataJson.decode(response.json.getJSONObject("data")))
                if (!repository.acceptRemote(remote, response.status != 409)) return@withContext false
                val updated = repository.state.value
                if (updated.data == updated.base.data) return@withContext true
            }
            true
        }
    }

    suspend fun watchRevisions(onRevision: (Long) -> Unit) {
        var delayMs = 1_000L
        while (currentCoroutineContext().isActive) {
            val revisions = Channel<Long>(Channel.BUFFERED)
            var socket: WebSocket? = null
            try {
                val access = token()
                val owner = withContext(Dispatchers.IO) { identity(access) }
                val ticket = withContext(Dispatchers.IO) {
                    val response = request("POST", "$API/api/sync/events-ticket", access)
                    if (response.status !in 200..299) throw SyncHttpException(response.status, "events")
                    response.json
                }
                val address = okhttp3.HttpUrl.Builder()
                    .scheme("https").host(URI(API).host)
                    .addPathSegments("api/sync/events")
                    .addQueryParameter("owner", owner)
                    .addQueryParameter("ticket", ticket.getString("ticket"))
                    .addQueryParameter("signature", ticket.getString("signature"))
                    .build()
                socket = socketClient.newWebSocket(Request.Builder().url(address).build(), object : WebSocketListener() {
                    override fun onOpen(webSocket: WebSocket, response: SocketResponse) { revisions.trySend(-1) }
                    override fun onMessage(webSocket: WebSocket, text: String) {
                        runCatching { JSONObject(text).getLong("revision") }
                            .onSuccess { revisions.trySend(it) }
                    }
                    override fun onClosed(webSocket: WebSocket, code: Int, reason: String) { revisions.close() }
                    override fun onFailure(webSocket: WebSocket, t: Throwable, response: SocketResponse?) { revisions.close() }
                })
                for (revision in revisions) {
                    delayMs = 1_000L
                    if (revision < 0 || repository.state.value.base.revision != revision) onRevision(revision)
                }
            } catch (cancel: kotlinx.coroutines.CancellationException) {
                throw cancel
            } catch (_: Exception) {
                // A reconnect obtains a fresh Clerk token and a fresh one-use ticket.
            } finally {
                socket?.close(1000, "Stopped")
                revisions.close()
            }
            delay(delayMs)
            delayMs = (delayMs * 2).coerceAtMost(30_000L)
        }
    }

    suspend fun signOut() = mutex.withLock {
        awaitClerk()
        when (val result = Clerk.auth.signOut()) {
            is ClerkResult.Success -> Unit
            is ClerkResult.Failure -> error(result.errorMessage)
        }
    }
}
