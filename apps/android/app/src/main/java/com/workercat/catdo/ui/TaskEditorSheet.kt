package com.workercat.catdo.ui

import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.keyframes
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.workercat.catdo.data.AppData
import com.workercat.catdo.data.Recurrence
import com.workercat.catdo.data.Task
import com.workercat.catdo.ui.kirakira.*
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneOffset

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TaskEditorSheet(
    task: Task, data: AppData, isNew: Boolean,
    onDismiss: () -> Unit, onSave: (Task) -> Unit, onDelete: () -> Unit,
    onAddSubtask: (Task) -> Unit, onEditSubtask: (Task) -> Unit, onCompleteSubtask: (Task) -> Unit,
) {
    val kk = Kirakira.colors
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
        containerColor = kk.paper, contentColor = kk.ink, tonalElevation = 0.dp,
        shape = RoundedCornerShape(topStart = 28.dp, topEnd = 28.dp),
        scrimColor = Color.Black.copy(alpha = 0.5f),
        dragHandle = { SheetHandle() }) {
        Column(Modifier.fillMaxWidth().imePadding().animateContentSize()) {
            Column(Modifier.weight(1f, fill = false).verticalScroll(rememberScrollState()).padding(horizontal = 24.dp)) {
                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                    if (!isNew) PopIconButton(Icons.Outlined.DeleteOutline, "Delete task", { deleteConfirmation = true },
                        tint = kk.mutedInk, iconSize = 20.dp)
                    Spacer(Modifier.weight(1f))
                    PopIconButton(Icons.Outlined.Close, "Close task", onDismiss, iconSize = 20.dp)
                }
                // Long titles wrap instead of scrolling sideways; Enter moves on and pasted line breaks become spaces.
                BasicTextField(value = draft.title, onValueChange = { draft = draft.copy(title = it.replace('\n', ' ')) }, maxLines = 4,
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Next),
                    modifier = Modifier.fillMaxWidth().padding(vertical = 6.dp).semantics { contentDescription = "Task title" },
                    textStyle = MaterialTheme.typography.titleLarge.copy(color = kk.ink),
                    cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
                    decorationBox = { inner -> Box {
                        if (draft.title.isEmpty()) Text("What needs doing?", style = MaterialTheme.typography.titleLarge,
                            color = kk.mutedInk.copy(alpha = 0.8f))
                        inner()
                    } })
                BasicTextField(value = draft.notes, onValueChange = { draft = draft.copy(notes = it) },
                    modifier = Modifier.fillMaxWidth().heightIn(min = 56.dp).padding(top = 6.dp, bottom = 16.dp)
                        .semantics { contentDescription = "Notes" },
                    textStyle = MaterialTheme.typography.bodyMedium.copy(color = kk.mutedInk),
                    cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
                    decorationBox = { inner -> Box {
                        if (draft.notes.isEmpty()) Text("Add notes…", style = MaterialTheme.typography.bodyMedium,
                            color = kk.mutedInk.copy(alpha = 0.8f))
                        inner()
                    } })
                if (!isNew) {
                    val subtasks = data.tasks.filter { it.parentId == task.id }
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        if (subtasks.isNotEmpty()) Text("Subtasks · ${subtasks.size}", modifier = Modifier.weight(1f),
                            style = MaterialTheme.typography.labelMedium, color = kk.mutedInk)
                        PopButton(onClick = {
                            if (draft != task && draft.title.isNotBlank()) onSave(draft)
                            onAddSubtask(draft)
                        }, variant = PopVariant.Secondary, size = PopSize.Small, icon = Icons.Outlined.Add) { Text("Add subtask") }
                    }
                    Column(Modifier.offset(x = (-16).dp)) {
                        subtasks.forEach { child ->
                            TaskRow(child, data, {
                                if (draft != task && draft.title.isNotBlank()) onSave(draft)
                                onEditSubtask(child.copy(workspaceId = draft.workspaceId, projectId = draft.projectId))
                            }, { onCompleteSubtask(child) })
                        }
                    }
                }
                Spacer(Modifier.height(12.dp))
                // The properties sit in one card, each row a labelled option.
                Column(Modifier.popCard().padding(vertical = 4.dp)) {
                    Box {
                        EditorOption(Icons.Outlined.FolderOpen, "Project", projects.firstOrNull { it.id == draft.projectId }?.name ?: "Inbox",
                            hue = draft.projectId?.let { Accents.project(it) }) {
                            if (draft.parentId == null) projectMenu = true
                        }
                        PopMenu(projectMenu, { projectMenu = false }) {
                            PopMenuItem("Inbox", draft.projectId == null) { draft = draft.copy(projectId = null, parentId = null); projectMenu = false }
                            projects.forEach { project -> PopMenuItem(project.name, draft.projectId == project.id, Accents.project(project.id)) {
                                draft = draft.copy(projectId = project.id, parentId = null); projectMenu = false
                            } }
                        }
                    }
                    OptionDivider()
                    EditorOption(Icons.Outlined.EventAvailable, "Planned for", draft.scheduled?.let(::formatDay) ?: "Any day",
                        hue = Accents.scheduled().text,
                        onClear = if (draft.scheduled != null) ({ draft = draft.copy(scheduled = null, recurrence = null) }) else null,
                        clearLabel = "Clear planned date") {
                        pickDate(draft.scheduled) { date -> draft = draft.copy(
                            scheduled = date,
                            recurrence = if (draft.recurrence?.unit == "Months" && date != null)
                                draft.recurrence?.copy(monthDay = LocalDate.parse(date).dayOfMonth) else draft.recurrence,
                        ) }
                    }
                    OptionDivider()
                    val late = draft.due != null && draft.due!! < LocalDate.now().toString()
                    EditorOption(Icons.Outlined.Flag, "Deadline", draft.due?.let(::formatDay) ?: "None",
                        hue = if (late) Accents.overdue().text else Accents.today().text,
                        onClear = if (draft.due != null) ({ draft = draft.copy(due = null, recurrence = if (draft.scheduled == null) null else draft.recurrence) }) else null,
                        clearLabel = "Clear deadline") {
                        pickDate(draft.due) { date -> draft = draft.copy(
                            due = date,
                            recurrence = if (draft.scheduled == null && draft.recurrence?.unit == "Months" && date != null)
                                draft.recurrence?.copy(monthDay = LocalDate.parse(date).dayOfMonth) else draft.recurrence,
                        ) }
                    }
                    OptionDivider()
                    Box {
                        val repeatLabel = when (draft.recurrence?.unit) {
                            "Days" -> "Daily"
                            "Weeks" -> "Weekly"
                            "Months" -> "Monthly"
                            else -> "Never"
                        }
                        EditorOption(Icons.Outlined.Repeat, "Repeat", repeatLabel, hue = Accents.repeat().text) { repeatMenu = true }
                        PopMenu(repeatMenu, { repeatMenu = false }) {
                            listOf("Never", "Daily", "Weekly", "Monthly").forEach { label ->
                                PopMenuItem(label, label == repeatLabel) {
                                    val date = draft.scheduled ?: draft.due ?: LocalDate.now().toString()
                                    val rule = when (label) {
                                        "Daily" -> Recurrence("Days")
                                        "Weekly" -> Recurrence("Weeks")
                                        "Monthly" -> Recurrence("Months", monthDay = LocalDate.parse(date).dayOfMonth)
                                        else -> null
                                    }
                                    draft = draft.copy(scheduled = if (rule != null && draft.due == null) date else draft.scheduled, recurrence = rule)
                                    repeatMenu = false
                                }
                            }
                        }
                    }
                }
                Spacer(Modifier.height(16.dp))
            }
            HorizontalDivider(color = kk.border)
            PopButton(onClick = { onSave(draft) }, enabled = draft.title.trim().isNotBlank(), size = PopSize.Large,
                modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp, vertical = 12.dp)) { Text(if (isNew) "Add task" else "Save changes") }
        }
    }

    datePicked?.let { onPicked ->
        val picker = rememberDatePickerState(initialSelectedDateMillis = initialDate.atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli())
        PopDialog(onDismissRequest = { datePicked = null }, width = 380.dp,
            modifier = Modifier.padding(horizontal = 0.dp),
            buttons = { dialog ->
                PopButton(onClick = { dialog.dismiss { datePicked = null } }, variant = PopVariant.Ghost) { Text("Cancel") }
                PopButton(onClick = {
                    dialog.dismiss {
                        picker.selectedDateMillis?.let { onPicked(Instant.ofEpochMilli(it).atZone(ZoneOffset.UTC).toLocalDate().toString()) }
                        datePicked = null
                    }
                }, enabled = picker.selectedDateMillis != null) { Text("Done") }
            },
        ) {
            DatePicker(state = picker, modifier = Modifier.offset(x = (-12).dp).requiredWidth(344.dp),
                title = null, headline = null, showModeToggle = false, colors = popDatePickerColors())
        }
    }

    if (deleteConfirmation) PopDialog(
        onDismissRequest = { deleteConfirmation = false }, title = "Delete task?",
        buttons = { dialog ->
            PopButton(onClick = { dialog.dismiss { deleteConfirmation = false } }, variant = PopVariant.Ghost) { Text("Cancel") }
            PopButton(onClick = { dialog.dismiss(onDelete) }, variant = PopVariant.Destructive) { Text("Delete") }
        },
    ) {
        Text("This also removes its subtasks. You can undo the action right after deleting.", style = MaterialTheme.typography.bodyMedium)
    }
}

/** Pop Drawer's handle: it squashes and stretches as the sheet lands (1.3x0.6, 0.9x1.2, 1.05x0.95, 1). */
@Composable
private fun SheetHandle() {
    val kk = Kirakira.colors
    val reduced = Kirakira.reducedMotion
    val sx = remember { Animatable(1f) }
    val sy = remember { Animatable(1f) }
    LaunchedEffect(Unit) {
        if (reduced) return@LaunchedEffect
        coroutineScope {
            launch { sx.animateTo(1f, keyframes { durationMillis = 680; 1f at 280; 1.3f at 400; 0.9f at 520; 1.05f at 600 }) }
            launch { sy.animateTo(1f, keyframes { durationMillis = 680; 1f at 280; 0.6f at 400; 1.2f at 520; 0.95f at 600 }) }
        }
    }
    Box(Modifier.padding(top = 12.dp, bottom = 4.dp).size(40.dp, 5.dp)
        .graphicsLayer { scaleX = sx.value; scaleY = sy.value }
        .background(kk.mutedInk.copy(alpha = 0.35f), CircleShape))
}

@Composable
private fun OptionDivider() = HorizontalDivider(Modifier.padding(start = 58.dp, end = 12.dp), color = Kirakira.colors.border)

@Composable
private fun EditorOption(icon: ImageVector, title: String, value: String, hue: Color? = null,
    onClear: (() -> Unit)? = null, clearLabel: String = "Clear", onClick: () -> Unit) {
    val kk = Kirakira.colors
    val set = value != "Inbox" && value != "Any day" && value != "None" && value != "Never"
    val tone = if (set && hue != null) hue else kk.mutedInk
    Row(Modifier.fillMaxWidth().popClickable(onClick, sink = 0.99f).heightIn(min = 52.dp).padding(start = 12.dp, end = 4.dp),
        verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(32.dp).background(if (set) tone.copy(alpha = 0.14f) else kk.muted, RoundedCornerShape(10.dp)),
            contentAlignment = Alignment.Center) {
            Icon(icon, null, Modifier.size(17.dp), tint = tone)
        }
        Text(title, Modifier.weight(1f).padding(start = 14.dp), style = MaterialTheme.typography.bodyMedium, color = kk.mutedInk)
        Text(value, Modifier.widthIn(max = 150.dp).padding(start = 8.dp), maxLines = 2, overflow = TextOverflow.Ellipsis,
            style = MaterialTheme.typography.bodyMedium, color = if (set) kk.ink else kk.mutedInk)
        if (onClear != null) PopIconButton(Icons.Outlined.Close, clearLabel, onClear, tint = kk.mutedInk, iconSize = 16.dp)
        else Box(Modifier.size(width = 36.dp, height = 48.dp), contentAlignment = Alignment.Center) {
            Icon(Icons.Outlined.ChevronRight, null, Modifier.size(18.dp), tint = kk.mutedInk)
        }
    }
}

/** A Material dropdown dressed as Pop Dropdown Menu: a white card with a hairline and soft corners. */
@Composable
private fun PopMenu(expanded: Boolean, onDismiss: () -> Unit, content: @Composable ColumnScope.() -> Unit) {
    val kk = Kirakira.colors
    DropdownMenu(expanded, onDismiss, shape = RoundedCornerShape(18.dp), containerColor = kk.card,
        tonalElevation = 0.dp, shadowElevation = 8.dp, border = androidx.compose.foundation.BorderStroke(1.dp, kk.border),
        modifier = Modifier.widthIn(min = 200.dp), content = content)
}

@Composable
private fun PopMenuItem(label: String, chosen: Boolean, dot: Color? = null, onClick: () -> Unit) {
    val kk = Kirakira.colors
    DropdownMenuItem(
        text = { Text(label, style = MaterialTheme.typography.bodyMedium, color = kk.ink) },
        onClick = onClick,
        leadingIcon = dot?.let { { Box(Modifier.size(10.dp).background(it, CircleShape)) } },
        trailingIcon = if (chosen) ({ Icon(Icons.Outlined.Check, null, Modifier.size(18.dp).popIn(from = 0f), tint = MaterialTheme.colorScheme.primary) }) else null,
        modifier = Modifier.padding(horizontal = 6.dp).background(if (chosen) kk.blush else Color.Transparent, RoundedCornerShape(12.dp)),
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun popDatePickerColors(): DatePickerColors {
    val kk = Kirakira.colors
    val today = Accents.today()
    return DatePickerDefaults.colors(
        containerColor = Color.Transparent,
        titleContentColor = kk.mutedInk, headlineContentColor = kk.ink,
        weekdayContentColor = kk.mutedInk, subheadContentColor = kk.mutedInk,
        navigationContentColor = kk.ink, yearContentColor = kk.ink,
        currentYearContentColor = MaterialTheme.colorScheme.primary,
        selectedYearContentColor = kk.onPrimary, selectedYearContainerColor = kk.primary,
        dayContentColor = kk.ink, selectedDayContentColor = kk.onPrimary, selectedDayContainerColor = kk.primary,
        todayContentColor = today.text, todayDateBorderColor = today.text.copy(alpha = 0.6f),
        dayInSelectionRangeContainerColor = kk.blush, dayInSelectionRangeContentColor = kk.ink,
        dividerColor = kk.border,
    )
}
