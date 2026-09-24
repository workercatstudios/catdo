package com.workercat.catdo.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.ChevronLeft
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.workercat.catdo.data.*
import java.time.LocalDate
import java.time.YearMonth
import java.time.format.DateTimeFormatter

@Composable
fun CalendarScreen(data: AppData, workspace: Workspace, vm: CatDoViewModel) {
    var selected by remember { mutableStateOf(LocalDate.now()) }
    var month by remember { mutableStateOf(YearMonth.from(selected)) }
    val tasks = data.tasks.filter { it.workspaceId == workspace.id && it.isActive(data) }
    val agenda = tasks.filter { it.scheduled == selected.toString() || it.due == selected.toString() }
    val first = month.atDay(1)
    val offset = first.dayOfWeek.value - 1
    val dayCount = month.lengthOfMonth()
    val weeks = (offset + dayCount + 6) / 7

    LazyColumn(contentPadding = PaddingValues(bottom = 100.dp)) {
        item {
            SectionHeading("Calendar")
            Column(Modifier.padding(horizontal = 20.dp)) {
                Column {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(month.format(DateTimeFormatter.ofPattern("MMMM yyyy")), modifier = Modifier.weight(1f),
                            style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                        IconButton(onClick = {
                            month = month.minusMonths(1)
                            selected = month.atDay(minOf(selected.dayOfMonth, month.lengthOfMonth()))
                        }) { Icon(Icons.Outlined.ChevronLeft, "Previous month") }
                        IconButton(onClick = {
                            month = month.plusMonths(1)
                            selected = month.atDay(minOf(selected.dayOfMonth, month.lengthOfMonth()))
                        }) { Icon(Icons.Outlined.ChevronRight, "Next month") }
                    }
                    Spacer(Modifier.height(4.dp))
                    Row(Modifier.fillMaxWidth()) {
                        listOf("Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday").forEach { day ->
                            Box(Modifier.weight(1f).height(28.dp).semantics { contentDescription = day }, contentAlignment = Alignment.Center) {
                                Text(day.take(2), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                        }
                    }
                    repeat(weeks) { week ->
                        Row(Modifier.fillMaxWidth()) {
                            repeat(7) { weekday ->
                                val number = week * 7 + weekday - offset + 1
                                val date = if (number in 1..dayCount) month.atDay(number) else null
                                val hasTask = date != null && tasks.any { it.scheduled == date.toString() || it.due == date.toString() }
                                Box(Modifier.weight(1f).height(48.dp)
                                    .then(if (date != null) Modifier.clickable { selected = date }.semantics {
                                        this.selected = date == selected
                                        contentDescription = date.format(DateTimeFormatter.ofPattern("EEEE, MMMM d")) +
                                            if (hasTask) ", has tasks" else ""
                                    } else Modifier), contentAlignment = Alignment.Center) {
                                    if (date != null) {
                                        val active = date == selected
                                        Surface(
                                            shape = MaterialTheme.shapes.small,
                                            border = if (date == LocalDate.now() && !active) BorderStroke(1.dp, MaterialTheme.colorScheme.primary) else null,
                                            color = if (active) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.background,
                                            modifier = Modifier.size(32.dp),
                                        ) {
                                            Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.Center) {
                                                Text(number.toString(), style = MaterialTheme.typography.bodyMedium,
                                                    fontWeight = if (date == LocalDate.now()) FontWeight.Bold else FontWeight.Normal,
                                                    color = if (active) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onBackground)
                                                if (hasTask) Box(Modifier.size(4.dp)) {
                                                    Surface(Modifier.fillMaxSize(), shape = CircleShape,
                                                        color = if (active) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.primary) {}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Spacer(Modifier.height(16.dp))
                HorizontalDivider()
                Spacer(Modifier.height(12.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.weight(1f)) {
                        Text(selected.format(DateTimeFormatter.ofPattern("EEEE, MMMM d")), style = MaterialTheme.typography.labelLarge,
                            fontWeight = FontWeight.Medium)
                        Text("${agenda.size} ${if (agenda.size == 1) "task" else "tasks"}",
                            color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodySmall)
                    }
                    Button(onClick = { vm.newTask(selected.toString()) }, shape = MaterialTheme.shapes.small,
                        contentPadding = PaddingValues(horizontal = 12.dp), modifier = Modifier.height(36.dp)) {
                        Text("Add task", style = MaterialTheme.typography.labelMedium)
                    }
                }
            }
        }
        if (agenda.isEmpty()) item {
            Text("No tasks planned for this day.", modifier = Modifier.padding(horizontal = 24.dp, vertical = 28.dp),
                color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        items(agenda, key = { it.id }) { task -> TaskRow(task, data, { vm.edit(task) }, { vm.complete(task.id) }, showScheduled = false) }
    }
}
