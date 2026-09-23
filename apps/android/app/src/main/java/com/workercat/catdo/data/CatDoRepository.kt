package com.workercat.catdo.data

import android.content.Context
import android.util.AtomicFile
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File
import java.util.UUID

data class Snapshot(val revision: Long, val data: AppData)
data class Pending(val id: String, val base: AppData, val data: AppData)
data class StoredState(
    val data: AppData,
    val owner: String? = null,
    val base: Snapshot = Snapshot(0, AppData.empty()),
    val pending: Pending? = null,
    val conflict: Snapshot? = null,
)

class CatDoRepository(context: Context) {
    private val file = AtomicFile(File(context.filesDir, "catdo.json"))
    private val mutex = Mutex()
    private val _state = MutableStateFlow(load())
    val state = _state.asStateFlow()
    private val _undo = MutableStateFlow<AppData?>(null)
    val canUndo = _undo.asStateFlow()

    private fun load(): StoredState = try {
        val root = JSONObject(file.openRead().bufferedReader().use { it.readText() })
        require(root.optInt("version", 1) == 1) { "This task file was made by a newer CatDo version." }
        StoredState(
            data = DataJson.decode(root.getJSONObject("data")).validate(),
            owner = if (root.isNull("owner")) null else root.getString("owner"),
            base = root.optJSONObject("base")?.let { Snapshot(it.getLong("revision"), DataJson.decode(it.getJSONObject("data"))) }
                ?: Snapshot(0, AppData.empty()),
            pending = root.optJSONObject("pending")?.let { Pending(it.getString("id"), DataJson.decode(it.getJSONObject("base")), DataJson.decode(it.getJSONObject("data"))) },
            conflict = root.optJSONObject("conflict")?.let { Snapshot(it.getLong("revision"), DataJson.decode(it.getJSONObject("data"))) },
        )
    } catch (_: java.io.FileNotFoundException) {
        StoredState(AppData.initial())
    }

    private suspend fun persist(value: StoredState) = withContext(Dispatchers.IO) {
        val root = JSONObject().apply {
            put("version", 1)
            put("data", DataJson.encode(value.data))
            put("owner", value.owner ?: JSONObject.NULL)
            put("base", JSONObject().put("revision", value.base.revision).put("data", DataJson.encode(value.base.data)))
            put("pending", value.pending?.let { JSONObject().put("id", it.id).put("base", DataJson.encode(it.base)).put("data", DataJson.encode(it.data)) } ?: JSONObject.NULL)
            put("conflict", value.conflict?.let { JSONObject().put("revision", it.revision).put("data", DataJson.encode(it.data)) } ?: JSONObject.NULL)
        }
        val stream = file.startWrite()
        try {
            stream.write(root.toString().toByteArray(Charsets.UTF_8))
            file.finishWrite(stream)
        } catch (error: Exception) {
            file.failWrite(stream)
            throw error
        }
    }

    suspend fun change(edit: (AppData) -> AppData) = mutex.withLock {
        val before = _state.value
        val after = withContext(Dispatchers.Default) { edit(before.data).validate() }
        if (after != before.data) {
            persist(before.copy(data = after))
            _undo.value = before.data
            _state.value = before.copy(data = after)
        }
    }

    suspend fun undo() = mutex.withLock {
        val previous = _undo.value ?: return@withLock
        val next = _state.value.copy(data = previous)
        persist(next)
        _state.value = next
        _undo.value = null
    }

    suspend fun replace(expected: StoredState, value: StoredState): Boolean = mutex.withLock {
        if (_state.value != expected) return@withLock false
        persist(value)
        _state.value = value
        true
    }

    suspend fun bindOwner(userId: String) = mutex.withLock {
        val current = _state.value
        require(current.owner == null || current.owner == userId) { "This device's task data belongs to another account." }
        if (current.owner == null) {
            val next = current.copy(owner = userId)
            persist(next)
            _state.value = next
        }
    }

    suspend fun freezePending(): Pending = mutex.withLock {
        val current = _state.value
        require(current.conflict == null) { "Resolve the sync conflict first." }
        current.pending?.let { return@withLock it }
        val pending = Pending(UUID.randomUUID().toString(), current.base.data, current.data)
        val next = current.copy(pending = pending)
        persist(next)
        _state.value = next
        pending
    }

    suspend fun acceptRemote(remote: Snapshot, acknowledged: Boolean): Boolean = mutex.withLock {
        val current = _state.value
        val baseData = if (acknowledged) current.pending?.data ?: current.base.data else current.base.data
        val result = if (baseData.workspaces.isEmpty() && current.data.isPristine() && remote.data.workspaces.isNotEmpty())
            MergeResult(remote.data, emptyList()) else mergeData(baseData, current.data, remote.data)
        val merged = runCatching { result.data.validate() }.getOrNull()
        val next = if (result.conflicts.isNotEmpty() || merged == null)
            current.copy(pending = null, base = current.base.copy(data = baseData), conflict = remote)
        else current.copy(data = merged, base = remote, pending = null, conflict = null)
        persist(next)
        _state.value = next
        merged != null && result.conflicts.isEmpty()
    }

    suspend fun resolveConflict(preferRemote: Boolean) = mutex.withLock {
        val current = _state.value
        val remote = requireNotNull(current.conflict)
        val localChoice = mergeData(current.base.data, current.data, remote.data, false).data
        val remoteChoice = mergeData(current.base.data, current.data, remote.data, true).data
        val merged = if (runCatching { localChoice.validate(); remoteChoice.validate() }.isSuccess)
            if (preferRemote) remoteChoice else localChoice
        else if (preferRemote) remote.data else current.data
        merged.validate()
        val next = current.copy(data = merged, base = remote, pending = null, conflict = null)
        persist(next)
        _state.value = next
    }
}
