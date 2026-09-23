package com.workercat.catdo.ui

import android.app.DatePickerDialog
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.workercat.catdo.data.AppData
import com.workercat.catdo.data.Recurrence
import com.workercat.catdo.data.Task
import java.time.LocalDate

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TaskEditorSheet(
    task: Task, data: AppData, isNew: Boolean,
    onDismiss: () -> Unit, onSave: (Task) -> Unit, onDelete: () -> Unit,
    onAddSubtask: (Task) -> Unit, onEditSubtask: (Task) -> Unit, onCompleteSubtask: (Task) -> Unit,
) {
    var draft by remember(task.id) { mutableStateOf(task) }
    var projectMenu by remember { mutableStateOf(false) }
    var repeatMenu by remember { mutableStateOf(false) }
    var deleteConfirmation by remember { mutableStateOf(false) }
    val context = LocalContext.current
    val projects = data.projects.filter { it.workspaceId == draft.workspaceId && !it.archived }

    fun pickDate(current: String?, onPicked: (String?) -> Unit) {
        val start = current?.let { runCatching { LocalDate.parse(it) }.getOrNull() } ?: LocalDate.now()
        DatePickerDialog(context, { _, year, month, day -> onPicked(LocalDate.of(year, month + 1, day).toString()) },
            start.year, start.monthValue - 1, start.dayOfMonth).show()
    }

    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)) {
        Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).imePadding().padding(horizontal = 24.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(if (isNew) "New task" else "Task details", style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.SemiBold, modifier = Modifier.weight(1f))
                if (!isNew) IconButton(onClick = { deleteConfirmation = true }) {
                    Icon(Icons.Outlined.DeleteOutline, "Delete task", tint = MaterialTheme.colorScheme.error)
                }
            }
            Spacer(Modifier.height(16.dp))
            OutlinedTextField(
                value = draft.title, onValueChange = { draft = draft.copy(title = it) },
                modifier = Modifier.fillMaxWidth(), label = { Text("Task") }, placeholder = { Text("What needs doing?") },
                singleLine = true, textStyle = MaterialTheme.typography.titleMedium,
            )
            Spacer(Modifier.height(12.dp))
            OutlinedTextField(
                value = draft.notes, onValueChange = { draft = draft.copy(notes = it) },
                modifier = Modifier.fillMaxWidth().heightIn(min = 100.dp), label = { Text("Notes") },
                placeholder = { Text("Add a little context") }, minLines = 3,
            )
            if (!isNew) {
                Spacer(Modifier.height(22.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("SUBTASKS", modifier = Modifier.weight(1f), style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary, fontWeight = FontWeight.Bold)
                    TextButton(onClick = {
                        if (draft != task && draft.title.isNotBlank()) onSave(draft)
                        onAddSubtask(draft)
                    }) { Text("Add subtask") }
                }
                data.tasks.filter { it.parentId == task.id }.forEach { child ->
                    TaskRow(child, data, {
                        if (draft != task && draft.title.isNotBlank()) onSave(draft)
                        onEditSubtask(child.copy(workspaceId = draft.workspaceId, projectId = draft.projectId))
                    }, { onCompleteSubtask(child) })
                }
            }
            Spacer(Modifier.height(22.dp))
            Text("ORGANIZE", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.primary,
                fontWeight = FontWeight.Bold)
            Spacer(Modifier.height(6.dp))
            Box {
                EditorOption(Icons.Outlined.FolderOpen, "Project", projects.firstOrNull { it.id == draft.projectId }?.name ?: "Inbox") {
                    if (draft.parentId == null) projectMenu = true
                }
                DropdownMenu(projectMenu, { projectMenu = false }) {
                    DropdownMenuItem(text = { Text("Inbox") }, onClick = { draft = draft.copy(projectId = null, parentId = null); projectMenu = false })
                    projects.forEach { project -> DropdownMenuItem(text = { Text(project.name) }, onClick = {
                        draft = draft.copy(projectId = project.id, parentId = null); projectMenu = false
                    }) }
                }
            }
            EditorOption(Icons.Outlined.EventAvailable, "Planned for", draft.scheduled?.let(::formatDay) ?: "Any day") {
                pickDate(draft.scheduled) { date -> draft = draft.copy(
                    scheduled = date,
                    recurrence = if (draft.recurrence?.unit == "Months" && date != null)
                        draft.recurrence?.copy(monthDay = LocalDate.parse(date).dayOfMonth) else draft.recurrence,
                ) }
            }
            if (draft.scheduled != null) TextButton(onClick = { draft = draft.copy(scheduled = null, recurrence = null) },
                modifier = Modifier.align(Alignment.End)) { Text("Clear planned date") }
            EditorOption(Icons.Outlined.Flag, "Deadline", draft.due?.let(::formatDay) ?: "None") {
                pickDate(draft.due) { date -> draft = draft.copy(
                    due = date,
                    recurrence = if (draft.scheduled == null && draft.recurrence?.unit == "Months" && date != null)
                        draft.recurrence?.copy(monthDay = LocalDate.parse(date).dayOfMonth) else draft.recurrence,
                ) }
            }
            if (draft.due != null) TextButton(onClick = { draft = draft.copy(due = null, recurrence = if (draft.scheduled == null) null else draft.recurrence) },
                modifier = Modifier.align(Alignment.End)) { Text("Clear deadline") }
            Box {
                EditorOption(Icons.Outlined.Repeat, "Repeat", when (draft.recurrence?.unit) {
                    "Days" -> "Daily"
                    "Weeks" -> "Weekly"
                    "Months" -> "Monthly"
                    else -> "Never"
                }) { repeatMenu = true }
                DropdownMenu(repeatMenu, { repeatMenu = false }) {
                    listOf("Never", "Daily", "Weekly", "Monthly").forEach { label ->
                        DropdownMenuItem(text = { Text(label) }, onClick = {
                            val date = draft.scheduled ?: draft.due ?: LocalDate.now().toString()
                            val rule = when (label) {
                                "Daily" -> Recurrence("Days")
                                "Weekly" -> Recurrence("Weeks")
                                "Monthly" -> Recurrence("Months", monthDay = LocalDate.parse(date).dayOfMonth)
                                else -> null
                            }
                            draft = draft.copy(scheduled = if (rule != null && draft.due == null) date else draft.scheduled, recurrence = rule)
                            repeatMenu = false
                        })
                    }
                }
            }
            Spacer(Modifier.height(28.dp))
            Button(onClick = { onSave(draft) }, enabled = draft.title.trim().isNotBlank(),
                modifier = Modifier.fillMaxWidth().height(54.dp)) { Text(if (isNew) "Add task" else "Save changes") }
            Spacer(Modifier.height(28.dp))
        }
    }

    if (deleteConfirmation) AlertDialog(
        onDismissRequest = { deleteConfirmation = false }, title = { Text("Delete task?") },
        text = { Text("This also removes its subtasks. You can undo the action right after deleting.") },
        confirmButton = { TextButton(onClick = onDelete) { Text("Delete", color = MaterialTheme.colorScheme.error) } },
        dismissButton = { TextButton(onClick = { deleteConfirmation = false }) { Text("Cancel") } },
    )
}

@Composable
private fun EditorOption(icon: androidx.compose.ui.graphics.vector.ImageVector, title: String, value: String, onClick: () -> Unit) {
    Row(Modifier.fillMaxWidth().clickable(onClick = onClick).heightIn(min = 58.dp), verticalAlignment = Alignment.CenterVertically) {
        Icon(icon, null, tint = MaterialTheme.colorScheme.primary)
        Text(title, Modifier.weight(1f).padding(start = 16.dp), style = MaterialTheme.typography.bodyMedium)
        Text(value, color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodyMedium)
        Icon(Icons.Outlined.ChevronRight, null, tint = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.size(20.dp))
    }
}
