package com.workercat.catdo.data

import org.json.JSONArray
import org.json.JSONObject
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.temporal.ChronoUnit
import java.util.UUID

data class Workspace(val id: String, val name: String, val archived: Boolean = false)
data class Project(val id: String, val workspaceId: String, val name: String, val archived: Boolean = false)
data class Recurrence(
    val unit: String,
    val interval: Int = 1,
    val afterCompletion: Boolean = false,
    val weekdays: List<Int> = emptyList(),
    val monthDay: Int = 1,
)
data class Reminder(val at: String, val delivered: Boolean = false)
data class Task(
    val id: String,
    val workspaceId: String,
    val projectId: String? = null,
    val parentId: String? = null,
    val title: String,
    val notes: String = "",
    val scheduled: String? = null,
    val due: String? = null,
    val recurrence: Recurrence? = null,
    val reminder: Reminder? = null,
    val completedAt: String? = null,
    val createdAt: String = Instant.now().toString(),
)
data class Completion(val id: String, val task: Task, val completedAt: String)
data class AppData(
    val workspaces: List<Workspace>,
    val projects: List<Project> = emptyList(),
    val tasks: List<Task> = emptyList(),
    val history: List<Completion> = emptyList(),
) {
    companion object {
        fun initial() = AppData(listOf(Workspace(UUID.randomUUID().toString(), "Personal")))
        fun empty() = AppData(emptyList())
    }
}

fun today(): String = LocalDate.now().toString()
fun newTask(workspaceId: String, title: String, projectId: String? = null, scheduled: String? = null) =
    Task(UUID.randomUUID().toString(), workspaceId, projectId, title = title.trim(), scheduled = scheduled)

fun Task.isActive(data: AppData): Boolean = completedAt == null &&
    data.workspaces.any { it.id == workspaceId && !it.archived } &&
    (projectId == null || data.projects.any { it.id == projectId && !it.archived })

fun Task.isToday(day: String = today()): Boolean =
    (scheduled != null && scheduled <= day) || (due != null && due <= day)

fun AppData.validate(): AppData {
    require(workspaces.size <= 100 && projects.size <= 2000 && tasks.size <= 10000 && history.size <= 20000)
    require(workspaces.any { !it.archived }) { "Keep at least one active workspace." }
    val ids = (workspaces.map { it.id } + projects.map { it.id } + tasks.map { it.id } + history.map { it.id })
    require(ids.size == ids.toSet().size) { "Duplicate record ID." }
    val workspaceIds = workspaces.map { it.id }.toSet()
    val projectsById = projects.associateBy { it.id }
    val tasksById = tasks.associateBy { it.id }
    projects.forEach { require(it.workspaceId in workspaceIds && it.name.isNotBlank() && it.name.length <= 200) }
    workspaces.forEach { require(it.name.isNotBlank() && it.name.length <= 200) }
    tasks.forEach { task ->
        require(task.workspaceId in workspaceIds && task.title.isNotBlank() && task.title.length <= 200 && task.notes.length <= 20000)
        require(task.projectId == null || projectsById[task.projectId]?.workspaceId == task.workspaceId)
        require(task.recurrence == null || task.scheduled != null || task.due != null)
        listOfNotNull(task.scheduled, task.due).forEach { LocalDate.parse(it) }
        val seen = mutableSetOf(task.id)
        var parentId = task.parentId
        while (parentId != null) {
            require(seen.add(parentId)) { "A task cannot be its own ancestor." }
            val parent = requireNotNull(tasksById[parentId]) { "Subtask parent is missing." }
            require(parent.workspaceId == task.workspaceId && parent.projectId == task.projectId)
            parentId = parent.parentId
        }
    }
    return this
}

fun AppData.complete(id: String, day: String = today()): AppData {
    val task = tasks.firstOrNull { it.id == id } ?: return this
    if (task.completedAt != null) return copy(tasks = tasks.map { if (it.id == id) it.copy(completedAt = null) else it })
    val completedAt = Instant.now().toString()
    val next = task.recurrence?.let { rule ->
        val anchor = LocalDate.parse(task.scheduled ?: task.due)
        val following = nextDate(rule, anchor, LocalDate.parse(day))
        val shift = ChronoUnit.DAYS.between(anchor, following)
        task.copy(
            scheduled = task.scheduled?.let { LocalDate.parse(it).plusDays(shift).toString() },
            due = task.due?.let { LocalDate.parse(it).plusDays(shift).toString() },
            reminder = task.reminder?.let { reminder ->
                val nextAt = Instant.parse(reminder.at).atZone(ZoneId.systemDefault()).plusDays(shift).toInstant()
                Reminder(nextAt.toString(), false)
            },
        )
    } ?: task.copy(completedAt = completedAt)
    return copy(
        tasks = tasks.map { if (it.id == id) next else it },
        history = history + Completion(UUID.randomUUID().toString(), task, completedAt),
    ).validate()
}

fun nextDate(rule: Recurrence, current: LocalDate, completed: LocalDate): LocalDate {
    require(rule.interval in 1..999)
    val threshold = if (rule.afterCompletion) completed else maxOf(current, completed)
    var candidate = if (rule.afterCompletion) completed else current
    repeat(400_000) {
        candidate = when (rule.unit) {
            "Months" -> {
                val month = candidate.withDayOfMonth(1).plusMonths(rule.interval.toLong())
                month.withDayOfMonth(minOf(month.lengthOfMonth(), if (rule.afterCompletion) completed.dayOfMonth else rule.monthDay))
            }
            "Weeks" -> candidate.plusWeeks(rule.interval.toLong())
            "Weekdays" -> candidate.plusDays(1)
            else -> candidate.plusDays(rule.interval.toLong())
        }
        if (candidate > threshold && (rule.unit != "Weekdays" || (candidate.dayOfWeek.value - 1) in rule.weekdays)) return candidate
    }
    error("Next occurrence is outside the supported date range.")
}

private fun JSONObject.optStringOrNull(key: String): String? = if (isNull(key)) null else getString(key)
private fun JSONObject.putNullable(key: String, value: Any?) = put(key, value ?: JSONObject.NULL)
private fun <T> JSONArray.asList(parse: (JSONObject) -> T): List<T> = (0 until length()).map { parse(getJSONObject(it)) }

object DataJson {
    fun encode(data: AppData): JSONObject = JSONObject().apply {
        put("workspaces", JSONArray().apply { data.workspaces.forEach { put(JSONObject().put("id", it.id).put("name", it.name).put("archived", it.archived)) } })
        put("projects", JSONArray().apply { data.projects.forEach { put(JSONObject().put("id", it.id).put("workspace_id", it.workspaceId).put("name", it.name).put("archived", it.archived)) } })
        put("tasks", JSONArray().apply { data.tasks.forEach { put(encodeTask(it)) } })
        put("history", JSONArray().apply { data.history.forEach { put(JSONObject().put("id", it.id).put("task", encodeTask(it.task)).put("completed_at", it.completedAt)) } })
    }

    fun decode(json: JSONObject): AppData = AppData(
        json.getJSONArray("workspaces").asList { Workspace(it.getString("id"), it.getString("name"), it.optBoolean("archived")) },
        json.getJSONArray("projects").asList { Project(it.getString("id"), it.getString("workspace_id"), it.getString("name"), it.optBoolean("archived")) },
        json.getJSONArray("tasks").asList(::decodeTask),
        json.getJSONArray("history").asList { Completion(it.getString("id"), decodeTask(it.getJSONObject("task")), it.getString("completed_at")) },
    )

    private fun encodeTask(task: Task) = JSONObject().apply {
        put("id", task.id); put("workspace_id", task.workspaceId)
        putNullable("project_id", task.projectId); putNullable("parent_id", task.parentId)
        put("title", task.title); put("notes", task.notes)
        putNullable("scheduled", task.scheduled); putNullable("due", task.due)
        putNullable("recurrence", task.recurrence?.let { rule -> JSONObject().apply {
            put("unit", rule.unit); put("interval", rule.interval); put("after_completion", rule.afterCompletion)
            put("weekdays", JSONArray(rule.weekdays)); put("month_day", rule.monthDay)
        } })
        putNullable("reminder", task.reminder?.let { JSONObject().put("at", it.at).put("delivered", it.delivered) })
        putNullable("completed_at", task.completedAt); put("created_at", task.createdAt)
    }

    private fun decodeTask(json: JSONObject): Task = Task(
        id = json.getString("id"), workspaceId = json.getString("workspace_id"),
        projectId = json.optStringOrNull("project_id"), parentId = json.optStringOrNull("parent_id"),
        title = json.getString("title"), notes = json.optString("notes", ""),
        scheduled = json.optStringOrNull("scheduled"), due = json.optStringOrNull("due"),
        recurrence = json.optJSONObject("recurrence")?.let { Recurrence(
            it.getString("unit"), it.getInt("interval"), it.getBoolean("after_completion"),
            (0 until it.getJSONArray("weekdays").length()).map { i -> it.getJSONArray("weekdays").getInt(i) }, it.getInt("month_day"),
        ) },
        reminder = json.optJSONObject("reminder")?.let { Reminder(it.getString("at"), it.optBoolean("delivered")) },
        completedAt = json.optStringOrNull("completed_at"), createdAt = json.getString("created_at"),
    )
}
