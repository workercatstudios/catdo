package com.workercat.catdo.sync

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.AtomicFile
import com.workercat.catdo.data.CatDoRepository
import com.workercat.catdo.data.DataJson
import com.workercat.catdo.data.Pending
import com.workercat.catdo.data.Snapshot
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File
import java.net.HttpURLConnection
import java.net.URI
import java.net.URL
import java.net.URLEncoder
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

private const val API = "https://catdo.workercat.com"
private const val KEY_ALIAS = "catdo-sync-credentials"

data class DeviceLogin(
    val clientId: String,
    val deviceCode: String,
    val userCode: String,
    val verificationUri: String,
    val verificationUriComplete: String?,
    val tokenEndpoint: String,
    val revocationEndpoint: String?,
    val expiresIn: Long,
    val interval: Long,
)

private data class Credentials(
    val clientId: String,
    val tokenEndpoint: String,
    val revocationEndpoint: String?,
    val accessToken: String,
    val refreshToken: String,
    val expiresAt: Long,
) {
    fun json() = JSONObject().apply {
        put("clientId", clientId); put("tokenEndpoint", tokenEndpoint)
        put("revocationEndpoint", revocationEndpoint ?: JSONObject.NULL)
        put("accessToken", accessToken); put("refreshToken", refreshToken); put("expiresAt", expiresAt)
    }
    companion object {
        fun parse(json: JSONObject) = Credentials(
            json.getString("clientId"), json.getString("tokenEndpoint"),
            if (json.isNull("revocationEndpoint")) null else json.getString("revocationEndpoint"),
            json.getString("accessToken"), json.getString("refreshToken"), json.getLong("expiresAt"),
        )
    }
}

private class CredentialStore(context: Context) {
    private val file = AtomicFile(File(context.filesDir, "credentials.json"))
    private fun key(): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey(KEY_ALIAS, null) as? SecretKey)?.let { return it }
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder(KEY_ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256).build())
        }.generateKey()
    }
    fun load(): Credentials? = try {
        val bytes = file.openRead().use { it.readBytes() }
        val nonce = bytes.copyOfRange(0, 12)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, nonce)) }
        Credentials.parse(JSONObject(String(cipher.doFinal(bytes.copyOfRange(12, bytes.size)), Charsets.UTF_8)))
    } catch (_: java.io.FileNotFoundException) { null }

    fun save(value: Credentials) {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.ENCRYPT_MODE, key()) }
        val bytes = cipher.iv + cipher.doFinal(value.json().toString().toByteArray(Charsets.UTF_8))
        val output = file.startWrite()
        try { output.write(bytes); file.finishWrite(output) }
        catch (error: Exception) { file.failWrite(output); throw error }
    }
    fun clear() { file.delete() }
}

private data class Response(val status: Int, val json: JSONObject)

private fun secure(url: String, issuer: String? = null): String {
    val uri = URI(url)
    require(uri.scheme == "https" && uri.host != null && uri.userInfo == null && uri.fragment == null) { "Sign-in requires a secure server URL." }
    if (issuer != null) require(URI(issuer).let { it.scheme == uri.scheme && it.host == uri.host && it.port == uri.port }) { "Unexpected sign-in endpoint." }
    return url
}

private fun request(method: String, address: String, bearer: String? = null, body: String? = null, form: Boolean = false): Response {
    secure(address)
    val connection = (URL(address).openConnection() as HttpURLConnection).apply {
        requestMethod = method
        connectTimeout = 20_000; readTimeout = 20_000
        instanceFollowRedirects = false
        setRequestProperty("Accept", "application/json")
        if (bearer != null) setRequestProperty("Authorization", "Bearer $bearer")
        if (body != null) {
            doOutput = true
            setRequestProperty("Content-Type", if (form) "application/x-www-form-urlencoded" else "application/json")
            outputStream.use { it.write(body.toByteArray(Charsets.UTF_8)) }
        }
    }
    return try {
        val status = connection.responseCode
        val input = if (status in 200..299) connection.inputStream else connection.errorStream
        val text = input?.bufferedReader()?.use { it.readText() } ?: "{}"
        Response(status, JSONObject(text))
    } finally { connection.disconnect() }
}

private fun form(vararg fields: Pair<String, String>) = fields.joinToString("&") {
    "${URLEncoder.encode(it.first, "UTF-8") }=${URLEncoder.encode(it.second, "UTF-8") }"
}

class SyncClient(context: Context, private val repository: CatDoRepository) {
    private val credentials = CredentialStore(context)
    private val mutex = Mutex()
    val signedIn: Boolean get() = credentials.load() != null

    suspend fun beginLogin(): DeviceLogin = withContext(Dispatchers.IO) {
        val config = request("GET", "$API/api/config").json
        val issuer = secure(config.getString("issuer")).trimEnd('/')
        val clientId = config.getString("desktopClientId")
        require(clientId.isNotBlank()) { "Sign-in is not configured on this server." }
        val discovery = request("GET", "$issuer/.well-known/oauth-authorization-server").json
        val endpoint = secure(discovery.getString("device_authorization_endpoint"), issuer)
        val tokenEndpoint = secure(discovery.getString("token_endpoint"), issuer)
        val revoke = discovery.optString("revocation_endpoint").takeIf { it.isNotBlank() }?.let { secure(it, issuer) }
        val result = request("POST", endpoint, body = form("client_id" to clientId, "scope" to "openid profile email offline_access"), form = true)
        require(result.status in 200..299) { "Could not start sign-in. Please try again." }
        val code = result.json
        DeviceLogin(clientId, code.getString("device_code"), code.getString("user_code"),
            secure(code.getString("verification_uri")), code.optString("verification_uri_complete").takeIf { it.isNotBlank() }?.let(::secure),
            tokenEndpoint, revoke, code.getLong("expires_in"), code.optLong("interval", 5).coerceAtLeast(1))
    }

    suspend fun finishLogin(login: DeviceLogin): String = withContext(Dispatchers.IO) {
        val deadline = System.currentTimeMillis() + login.expiresIn.coerceAtMost(3600) * 1000
        var interval = login.interval
        while (System.currentTimeMillis() < deadline) {
            delay(interval * 1000)
            val result = request("POST", login.tokenEndpoint, body = form(
                "client_id" to login.clientId,
                "grant_type" to "urn:ietf:params:oauth:grant-type:device_code",
                "device_code" to login.deviceCode,
            ), form = true)
            if (result.status in 200..299) {
                val token = result.json
                val access = token.getString("access_token")
                val user = identity(access)
                repository.bindOwner(user)
                credentials.save(Credentials(login.clientId, login.tokenEndpoint, login.revocationEndpoint,
                    access, token.getString("refresh_token"), System.currentTimeMillis() / 1000 + token.getLong("expires_in")))
                return@withContext user
            }
            when (result.json.optString("error")) {
                "authorization_pending" -> Unit
                "slow_down" -> interval += 5
                "access_denied" -> error("Sign-in was declined. Your local tasks are unchanged.")
                "expired_token" -> break
                else -> error("Could not complete sign-in. Please try again.")
            }
        }
        error("The sign-in code expired. Start sign-in again.")
    }

    private fun identity(accessToken: String): String {
        val response = request("GET", "$API/api/me", bearer = accessToken)
        require(response.status in 200..299) { "Sign in again to sync." }
        return response.json.getString("userId")
    }

    private fun token(): String {
        val current = requireNotNull(credentials.load()) { "Sign in to sync." }
        if (current.expiresAt > System.currentTimeMillis() / 1000 + 60) return current.accessToken
        val result = request("POST", secure(current.tokenEndpoint), body = form(
            "client_id" to current.clientId, "grant_type" to "refresh_token", "refresh_token" to current.refreshToken,
        ), form = true)
        require(result.status in 200..299) { "Your sign-in expired. Sign in again; your tasks are saved locally." }
        val refreshed = current.copy(
            accessToken = result.json.getString("access_token"),
            refreshToken = result.json.optString("refresh_token").ifBlank { current.refreshToken },
            expiresAt = System.currentTimeMillis() / 1000 + result.json.getLong("expires_in"),
        )
        credentials.save(refreshed)
        return refreshed.accessToken
    }

    suspend fun sync(): Boolean = mutex.withLock { withContext(Dispatchers.IO) {
        val access = token()
        val owner = identity(access)
        repository.bindOwner(owner)
        repeat(3) {
            val state = repository.state.value
            require(state.conflict == null) { "Resolve the sync conflict first." }
            val shouldPush = state.pending != null || state.data != state.base.data
            val pending = if (shouldPush) repository.freezePending() else null
            val response = if (pending == null) request("GET", "$API/api/sync", bearer = access)
            else request("POST", "$API/api/sync", bearer = access, body = JSONObject().apply {
                put("id", pending.id); put("base", DataJson.encode(pending.base)); put("data", DataJson.encode(pending.data))
            }.toString())
            require(response.status in 200..299 || response.status == 409) {
                "Sync is unavailable (${response.status}). Your changes are saved on this device."
            }
            val remote = Snapshot(response.json.getLong("revision"), DataJson.decode(response.json.getJSONObject("data")))
            if (!repository.acceptRemote(remote, response.status != 409)) return@withContext false
            val updated = repository.state.value
            if (updated.data == updated.base.data) return@withContext true
        }
        true
    } }

    suspend fun signOut() = mutex.withLock { withContext(Dispatchers.IO) {
        val current = credentials.load() ?: return@withContext
        credentials.clear()
        current.revocationEndpoint?.let { endpoint -> runCatching {
            request("POST", secure(endpoint), body = form("client_id" to current.clientId,
                "token" to current.refreshToken, "token_type_hint" to "refresh_token"), form = true)
        } }
    } }
}
