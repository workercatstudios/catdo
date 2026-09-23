package com.workercat.catdo.ui

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.workercat.catdo.sync.DeviceLogin
import com.workercat.catdo.sync.SyncClient
import com.workercat.catdo.data.AppData
import com.workercat.catdo.data.CatDoRepository
import com.workercat.catdo.data.Project
import com.workercat.catdo.data.Task
import com.workercat.catdo.data.Workspace
import com.workercat.catdo.data.complete
import com.workercat.catdo.data.newTask
import kotlinx.coroutines.launch
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import java.util.UUID

enum class Section { Today, Inbox, Upcoming, Calendar, Projects, Completed, Search, Settings }

class CatDoViewModel(val repository: CatDoRepository, private val syncClient: SyncClient) : ViewModel() {
    val state = repository.state
    val canUndo = repository.canUndo
    var section by mutableStateOf(Section.Today)
    var workspaceId by mutableStateOf<String?>(null)
    var projectId by mutableStateOf<String?>(null)
    var search by mutableStateOf("")
    var editor by mutableStateOf<Task?>(null)
    var creating by mutableStateOf(false)
    var nameDialog by mutableStateOf<String?>(null)
    var message by mutableStateOf<String?>(null)
    var undoPrompt by mutableStateOf<String?>(null)
    var signedIn by mutableStateOf(syncClient.signedIn)
    var syncing by mutableStateOf(false)
    var deviceLogin by mutableStateOf<DeviceLogin?>(null)
    private var syncJob: Job? = null
    private var syncRequested = false

    fun startLogin() = viewModelScope.launch {
        if (deviceLogin != null || syncing) return@launch
        syncing = true
        runCatching { syncClient.beginLogin() }.onSuccess { login ->
            deviceLogin = login
            viewModelScope.launch {
                runCatching { syncClient.finishLogin(login) }.onSuccess {
                    signedIn = true
                    deviceLogin = null
                }.onFailure { message = it.message ?: "Could not sign in."; deviceLogin = null }
            }
        }.onFailure { message = it.message ?: "Could not start sign-in." }
        syncing = false
    }

    fun sync() = viewModelScope.launch {
        if (!signedIn) return@launch
        if (syncing) { syncRequested = true; return@launch }
        do {
            syncRequested = false
            syncing = true
            runCatching { syncClient.sync() }.onFailure { message = it.message ?: "Could not sync. Your tasks are saved here." }
            syncing = false
        } while (syncRequested && signedIn)
    }

    fun signOut() = viewModelScope.launch {
        syncJob?.cancel()
        syncRequested = false
        runCatching { syncClient.signOut() }.onFailure { message = it.message ?: "Could not sign out." }
            .onSuccess { signedIn = false; message = "Signed out. Tasks remain on this device." }
    }

    fun resolveConflict(preferRemote: Boolean) = viewModelScope.launch {
        runCatching { repository.resolveConflict(preferRemote) }.onSuccess { sync() }
            .onFailure { message = it.message ?: "Could not resolve sync conflict." }
    }

    fun select(section: Section, projectId: String? = null) {
        this.section = section
        this.projectId = projectId
    }

    fun newTask(defaultDate: String? = null) {
        val workspace = workspaceId ?: state.value.data.workspaces.firstOrNull { !it.archived }?.id ?: return
        editor = newTask(workspace, "", projectId, defaultDate)
        creating = true
    }

    fun edit(task: Task) {
        editor = task
        creating = false
    }

    fun addSubtask(parent: Task) {
        editor = newTask(parent.workspaceId, "", parent.projectId).copy(parentId = parent.id)
        creating = true
    }

    fun save(task: Task) = mutate { data ->
        val trimmed = task.copy(title = task.title.trim(), notes = task.notes.trim())
        require(trimmed.title.isNotBlank()) { "Give this task a name." }
        val children = mutableSetOf(trimmed.id)
        while (true) {
            val before = children.size
            data.tasks.filter { it.parentId in children }.forEach { children += it.id }
            if (children.size == before) break
        }
        data.copy(tasks = if (data.tasks.any { it.id == trimmed.id }) data.tasks.map {
            when {
                it.id == trimmed.id -> trimmed
                it.id in children -> it.copy(workspaceId = trimmed.workspaceId, projectId = trimmed.projectId)
                else -> it
            }
        } else data.tasks + trimmed)
    }

    fun delete(id: String) = mutate("Task deleted") { data ->
        val children = mutableSetOf(id)
        var changed: Boolean
        do {
            val before = children.size
            data.tasks.filter { it.parentId in children }.forEach { children.add(it.id) }
            changed = children.size > before
        } while (changed)
        data.copy(tasks = data.tasks.filterNot { it.id in children })
    }

    fun complete(id: String) = mutate("Task completed") { it.complete(id) }
    fun undo() = viewModelScope.launch {
        runCatching { repository.undo() }.onSuccess { if (signedIn) sync() }
            .onFailure { message = it.message }
    }

    fun createWorkspace(name: String) {
        val id = UUID.randomUUID().toString()
        mutate(onSaved = { workspaceId = id; section = Section.Today }) { data ->
            require(name.trim().isNotBlank()) { "Enter a workspace name." }
            val workspace = Workspace(id, name.trim())
            data.copy(workspaces = data.workspaces + workspace)
        }
    }

    fun createProject(name: String) {
        val id = UUID.randomUUID().toString()
        val workspace = workspaceId ?: state.value.data.workspaces.first().id
        mutate(onSaved = { projectId = id; section = Section.Projects }) { data ->
            require(name.trim().isNotBlank()) { "Enter a project name." }
            val project = Project(id, workspace, name.trim())
            data.copy(projects = data.projects + project)
        }
    }

    fun renameWorkspace(id: String, name: String) = mutate { data ->
        require(name.trim().isNotBlank())
        data.copy(workspaces = data.workspaces.map { if (it.id == id) it.copy(name = name.trim()) else it })
    }

    fun renameProject(id: String, name: String) = mutate { data ->
        require(name.trim().isNotBlank())
        data.copy(projects = data.projects.map { if (it.id == id) it.copy(name = name.trim()) else it })
    }

    fun archiveProject(id: String) = mutate(onSaved = { projectId = null }) { data ->
        data.copy(projects = data.projects.map { if (it.id == id) it.copy(archived = true) else it })
    }

    private fun mutate(prompt: String? = null, onSaved: (() -> Unit)? = null, edit: (AppData) -> AppData) = viewModelScope.launch {
        runCatching { repository.change(edit) }.onSuccess {
            onSaved?.invoke()
            if (prompt != null) undoPrompt = prompt
            if (signedIn) {
                syncJob?.cancel()
                syncJob = viewModelScope.launch { delay(1200); sync() }
            }
        }.onFailure { message = it.message ?: "Could not save changes." }
    }
}
