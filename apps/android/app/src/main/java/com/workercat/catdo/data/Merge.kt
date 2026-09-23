package com.workercat.catdo.data

data class MergeResult(val data: AppData, val conflicts: List<String>)

fun AppData.isPristine() = workspaces.size == 1 && workspaces[0].name == "Personal" &&
    !workspaces[0].archived && projects.isEmpty() && tasks.isEmpty() && history.isEmpty()

fun mergeData(base: AppData, local: AppData, remote: AppData, preferRemote: Boolean? = null): MergeResult {
    val conflicts = mutableListOf<String>()
    val conflictingTasks = mutableSetOf<String>()
    fun <T> merge(
        before: List<T>, mine: List<T>, theirs: List<T>, id: (T) -> String, name: (T) -> String,
        tasks: Boolean = false,
    ): List<T> {
        val b = before.associateBy(id)
        val l = mine.associateBy(id)
        val r = theirs.associateBy(id)
        val order = (theirs + mine + before).map(id).distinct()
        return order.mapNotNull { key ->
            val old = b[key]
            val localValue = l[key]
            val remoteValue = r[key]
            when {
                localValue == old -> remoteValue
                remoteValue == old || localValue == remoteValue -> localValue
                else -> {
                    conflicts += name(requireNotNull(localValue ?: remoteValue))
                    if (tasks) conflictingTasks += key
                    if (preferRemote == true) remoteValue else localValue
                }
            }
        }
    }
    val workspaces = merge(base.workspaces, local.workspaces, remote.workspaces, { it.id }, { it.name })
    val projects = merge(base.projects, local.projects, remote.projects, { it.id }, { it.name })
    val tasks = merge(base.tasks, local.tasks, remote.tasks, { it.id }, { it.title }, true)
    var history = merge(base.history, local.history, remote.history, { it.id }, { it.task.title })
    val original = base.history.map { it.id }.toSet()
    val localAdded = local.history.filter { it.id !in original }
    val remoteAdded = remote.history.filter { it.id !in original }
    localAdded.forEach { mine ->
        if (remoteAdded.any { it.task.id == mine.task.id && it.id != mine.id }) {
            if (conflictingTasks.add(mine.task.id)) conflicts += mine.task.title
        }
    }
    val chosenHistory = (if (preferRemote == true) remote else local).history.map { it.id }.toSet()
    history = history.filter { it.task.id !in conflictingTasks || it.id in original || it.id in chosenHistory }
    return MergeResult(AppData(workspaces, projects, tasks, history), conflicts.distinct())
}
