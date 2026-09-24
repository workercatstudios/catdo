package com.workercat.catdo.ui

import androidx.compose.foundation.text.BasicTextField
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.foundation.clickable
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.workercat.catdo.data.AppData
import com.workercat.catdo.data.Recurrence
import com.workercat.catdo.data.Task
import java.time.LocalDate
import java.time.Instant
import java.time.ZoneOffset

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
    var initialDate by remember { mutableStateOf(LocalDate.now()) }
    var datePicked by remember { mutableStateOf<((String?) -> Unit)?>(null) }
    val projects = data.projects.filter { it.workspaceId == draft.workspaceId && !it.archived }

    fun pickDate(current: String?, onPicked: (String?) -> Unit) {
        initialDate = current?.let { runCatching { LocalDate.parse(it) }.getOrNull() } ?: LocalDate.now()
        datePicked = onPicked
    }

    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true),
        containerColor = MaterialTheme.colorScheme.background, tonalElevation = 0.dp) {
        Column(Modifier.fillMaxWidth().imePadding()) {
            Column(Modifier.weight(1f, fill = false).verticalScroll(rememberScrollState()).padding(horizontal = 24.dp)) {
                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                    if (!isNew) IconButton(onClick = { deleteConfirmation = true }) {
                        Icon(Icons.Outlined.DeleteOutline, "Delete task", Modifier.size(20.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    Spacer(Modifier.weight(1f))
                    IconButton(onClick = onDismiss) { Icon(Icons.Outlined.Close, "Close task", Modifier.size(20.dp)) }
                }
                BasicTextField(value = draft.title, onValueChange = { draft = draft.copy(title = it) }, singleLine = true,
                    modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp).semantics { contentDescription = "Task title" },
                    textStyle = MaterialTheme.typography.titleLarge.copy(color = MaterialTheme.colorScheme.onSurface),
                    cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
                    decorationBox = { inner -> Box {
                        if (draft.title.isEmpty()) Text("What needs doing?", style = MaterialTheme.typography.titleLarge,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                        inner()
                    } })
                BasicTextField(value = draft.notes, onValueChange = { draft = draft.copy(notes = it) },
                    modifier = Modifier.fillMaxWidth().heightIn(min = 56.dp).padding(top = 8.dp, bottom = 16.dp)
                        .semantics { contentDescription = "Notes" },
                    textStyle = MaterialTheme.typography.bodyMedium.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
                    cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
                    decorationBox = { inner -> Box {
                        if (draft.notes.isEmpty()) Text("Add notes…", style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                        inner()
                    } })
                if (!isNew) {
                    Spacer(Modifier.height(4.dp))
                    val subtasks = data.tasks.filter { it.parentId == task.id }
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        if (subtasks.isNotEmpty()) Text("Subtasks · ${subtasks.size}", modifier = Modifier.weight(1f),
                            style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        TextButton(onClick = {
                            if (draft != task && draft.title.isNotBlank()) onSave(draft)
                            onAddSubtask(draft)
                        }) { Text("Add subtask") }
                    }
                    subtasks.forEach { child ->
                        TaskRow(child, data, {
                            if (draft != task && draft.title.isNotBlank()) onSave(draft)
                            onEditSubtask(child.copy(workspaceId = draft.workspaceId, projectId = draft.projectId))
                        }, { onCompleteSubtask(child) })
                    }
                }
                Spacer(Modifier.height(16.dp))
                HorizontalDivider()
                Column {
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

                    EditorOption(Icons.Outlined.EventAvailable, "Planned for", draft.scheduled?.let(::formatDay) ?: "Any day",
                        onClear = if (draft.scheduled != null) ({ draft = draft.copy(scheduled = null, recurrence = null) }) else null,
                        clearLabel = "Clear planned date") {
                        pickDate(draft.scheduled) { date -> draft = draft.copy(
                            scheduled = date,
                            recurrence = if (draft.recurrence?.unit == "Months" && date != null)
                                draft.recurrence?.copy(monthDay = LocalDate.parse(date).dayOfMonth) else draft.recurrence,
                        ) }
                    }

                    EditorOption(Icons.Outlined.Flag, "Deadline", draft.due?.let(::formatDay) ?: "None",
                        onClear = if (draft.due != null) ({ draft = draft.copy(due = null, recurrence = if (draft.scheduled == null) null else draft.recurrence) }) else null,
                        clearLabel = "Clear deadline") {
                        pickDate(draft.due) { date -> draft = draft.copy(
                            due = date,
                            recurrence = if (draft.scheduled == null && draft.recurrence?.unit == "Months" && date != null)
                                draft.recurrence?.copy(monthDay = LocalDate.parse(date).dayOfMonth) else draft.recurrence,
                        ) }
                    }

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
                }
                Spacer(Modifier.height(20.dp))
            }
            HorizontalDivider()
            Button(onClick = { onSave(draft) }, enabled = draft.title.trim().isNotBlank(),
                shape = MaterialTheme.shapes.small, modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp, vertical = 16.dp).height(48.dp)) { Text(if (isNew) "Add task" else "Save changes") }
        }
    }

    datePicked?.let { onPicked ->
        val picker = rememberDatePickerState(initialSelectedDateMillis = initialDate.atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli())
        DatePickerDialog(onDismissRequest = { datePicked = null },
            confirmButton = { TextButton(onClick = {
                picker.selectedDateMillis?.let { onPicked(Instant.ofEpochMilli(it).atZone(ZoneOffset.UTC).toLocalDate().toString()) }
                datePicked = null
            }, enabled = picker.selectedDateMillis != null) { Text("Done") } },
            dismissButton = { TextButton(onClick = { datePicked = null }) { Text("Cancel") } },
        ) { DatePicker(state = picker) }
    }

    if (deleteConfirmation) AlertDialog(
        onDismissRequest = { deleteConfirmation = false }, title = { Text("Delete task?") },
        text = { Text("This also removes its subtasks. You can undo the action right after deleting.") },
        confirmButton = { TextButton(onClick = onDelete) { Text("Delete", color = MaterialTheme.colorScheme.error) } },
        dismissButton = { TextButton(onClick = { deleteConfirmation = false }) { Text("Cancel") } },
    )
}

@Composable
private fun EditorOption(icon: androidx.compose.ui.graphics.vector.ImageVector, title: String, value: String,
    onClear: (() -> Unit)? = null, clearLabel: String = "Clear", onClick: () -> Unit) {
    Row(Modifier.fillMaxWidth().clickable(onClick = onClick).heightIn(min = 48.dp), verticalAlignment = Alignment.CenterVertically) {
        Icon(icon, null, Modifier.size(18.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(title, Modifier.weight(1f).padding(start = 12.dp), style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value, Modifier.widthIn(max = 150.dp).padding(start = 8.dp), maxLines = 2, overflow = TextOverflow.Ellipsis,
            style = MaterialTheme.typography.bodyMedium)
        if (onClear != null) IconButton(onClick = onClear, modifier = Modifier.size(48.dp)) {
            Icon(Icons.Outlined.Close, clearLabel, Modifier.size(16.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
        } else Box(Modifier.width(32.dp), contentAlignment = Alignment.CenterEnd) {
            Icon(Icons.Outlined.ChevronRight, null, Modifier.size(18.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}
