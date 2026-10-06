package com.workercat.catdo.ui

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.keyframes
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.ChevronLeft
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.workercat.catdo.data.*
import com.workercat.catdo.ui.kirakira.*
import java.time.LocalDate
import java.time.YearMonth
import java.time.format.DateTimeFormatter

@Composable
fun CalendarScreen(data: AppData, workspace: Workspace, vm: CatDoViewModel) {
    val kk = Kirakira.colors
    val reduced = Kirakira.reducedMotion
    var selected by remember { mutableStateOf(LocalDate.now()) }
    var month by remember { mutableStateOf(YearMonth.from(selected)) }
    val tasks = data.tasks.filter { it.workspaceId == workspace.id && it.isActive(data) }
    val agenda = tasks.filter { it.scheduled == selected.toString() || it.due == selected.toString() }
    val seen = remember { mutableSetOf<String>() }

    LazyColumn(contentPadding = PaddingValues(bottom = 100.dp)) {
        item {
            SectionHeading("Calendar", icon = Section.Calendar.icon(), hue = Section.Calendar.hue())
            Column(Modifier.padding(horizontal = 16.dp).popCard().padding(horizontal = 8.dp, vertical = 6.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    // Pop Calendar's caption slides in from the side the new month comes from.
                    AnimatedContent(month, Modifier.weight(1f).padding(start = 12.dp), transitionSpec = {
                        val dir = if (targetState > initialState) 1 else -1
                        if (reduced) fadeIn(tween(240)) togetherWith fadeOut(tween(120))
                        else (slideInHorizontally(tween(360, easing = KkEase.Out)) { dir * it / 6 } + fadeIn(tween(240, 100))) togetherWith
                            (slideOutHorizontally(tween(150, easing = KkEase.CssIn)) { -dir * it / 8 } + fadeOut(tween(120)))
                    }, label = "caption") { shown ->
                        Text(shown.format(DateTimeFormatter.ofPattern("MMMM yyyy")), style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.ExtraBold, modifier = Modifier.semantics { heading() })
                    }
                    PopIconButton(Icons.Outlined.ChevronLeft, "Previous month", {
                        month = month.minusMonths(1)
                        selected = month.atDay(minOf(selected.dayOfMonth, month.lengthOfMonth()))
                    })
                    PopIconButton(Icons.Outlined.ChevronRight, "Next month", {
                        month = month.plusMonths(1)
                        selected = month.atDay(minOf(selected.dayOfMonth, month.lengthOfMonth()))
                    })
                }
                Row(Modifier.fillMaxWidth()) {
                    listOf("Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday").forEach { day ->
                        Box(Modifier.weight(1f).height(28.dp).semantics { contentDescription = day }, contentAlignment = Alignment.Center) {
                            Text(day.take(2), style = MaterialTheme.typography.labelSmall, color = kk.mutedInk)
                        }
                    }
                }
                // The weeks slide in from the side of the new month, overshoot a touch and settle.
                AnimatedContent(month, transitionSpec = {
                    val dir = if (targetState > initialState) 1 else -1
                    if (reduced) fadeIn(tween(240)) togetherWith fadeOut(tween(120))
                    else (slideInHorizontally(tween(400, easing = KkEase.Spring)) { dir * it * 2 / 5 } + fadeIn(tween(220, 80))) togetherWith
                        (slideOutHorizontally(tween(150, easing = KkEase.CssIn)) { -dir * it / 4 } + fadeOut(tween(150)))
                }, label = "weeks") { shown ->
                    val first = shown.atDay(1)
                    val offset = first.dayOfWeek.value - 1
                    val dayCount = shown.lengthOfMonth()
                    val weeks = (offset + dayCount + 6) / 7
                    Column {
                        repeat(weeks) { week ->
                            Row(Modifier.fillMaxWidth()) {
                                repeat(7) { weekday ->
                                    val number = week * 7 + weekday - offset + 1
                                    val date = if (number in 1..dayCount) shown.atDay(number) else null
                                    Box(Modifier.weight(1f).height(48.dp), contentAlignment = Alignment.Center) {
                                        if (date != null) CalendarDay(date, date == selected,
                                            hasTask = tasks.any { it.scheduled == date.toString() || it.due == date.toString() },
                                            hasDue = tasks.any { it.due == date.toString() }) { selected = date }
                                    }
                                }
                            }
                        }
                        Spacer(Modifier.height(4.dp))
                    }
                }
            }
            Row(Modifier.padding(start = 24.dp, end = 16.dp, top = 20.dp, bottom = 8.dp), verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text(selected.format(DateTimeFormatter.ofPattern("EEEE, MMMM d")), style = MaterialTheme.typography.titleSmall,
                        modifier = Modifier.semantics { heading() })
                    Text("${agenda.size} ${if (agenda.size == 1) "task" else "tasks"}",
                        color = kk.mutedInk, style = MaterialTheme.typography.bodySmall)
                }
                PopButton(onClick = { vm.newTask(selected.toString()) }, size = PopSize.Small, icon = Icons.Outlined.Add) { Text("Add task") }
            }
        }
        if (agenda.isEmpty()) item {
            Text("No tasks planned for this day.", modifier = Modifier.padding(horizontal = 24.dp, vertical = 24.dp),
                color = kk.mutedInk, style = MaterialTheme.typography.bodyMedium)
        }
        itemsIndexed(agenda, key = { _, task -> task.id }) { index, task ->
            TaskRow(task, data, { vm.edit(task) }, { vm.complete(task.id) }, showScheduled = false,
                modifier = Modifier.animateItem().rowEnter(task.id, index, seen))
        }
    }
}

/** A Pop Calendar day: squashes under the finger and pops when picked (0.55 -> 1.18 -> 0.95 -> 1). */
@Composable
private fun CalendarDay(date: LocalDate, active: Boolean, hasTask: Boolean, hasDue: Boolean, onClick: () -> Unit) {
    val kk = Kirakira.colors
    val reduced = Kirakira.reducedMotion
    val isToday = date == LocalDate.now()
    val today = Accents.today()
    val fill by animateColorAsState(
        when { active -> kk.primary; isToday -> today.fill.copy(alpha = 0.3f); else -> Color.Transparent },
        tween(150), label = "dayFill")
    val pick = remember { Animatable(1f) }
    var wasActive by remember { mutableStateOf(active) }
    LaunchedEffect(active) {
        if (active && !wasActive && !reduced) pick.animateTo(1f, keyframes {
            durationMillis = 400
            0.55f at 0 using KkEase.CssInOut
            1.18f at 180 using KkEase.CssInOut
            0.95f at 288 using KkEase.CssInOut
        })
        wasActive = active
    }
    Box(
        Modifier.size(44.dp)
            .popClickable(onClick, shape = CircleShape, sink = 0.86f, pressed = if (active) Color.Transparent else kk.muted, role = null)
            .semantics {
                selected = active
                contentDescription = date.format(DateTimeFormatter.ofPattern("EEEE, MMMM d")) + if (hasTask) ", has tasks" else ""
            },
        contentAlignment = Alignment.Center,
    ) {
        Box(
            Modifier.size(36.dp).graphicsLayer { scaleX = pick.value; scaleY = pick.value }
                .background(fill, CircleShape)
                .then(if (isToday && !active) Modifier.border(1.5.dp, today.text.copy(alpha = 0.6f), CircleShape) else Modifier),
            contentAlignment = Alignment.Center,
        ) {
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Text(date.dayOfMonth.toString(), style = MaterialTheme.typography.bodyMedium,
                    fontWeight = if (isToday || active) FontWeight.ExtraBold else FontWeight.Medium,
                    color = if (active) kk.onPrimary else if (isToday) today.text else kk.ink)
                Box(Modifier.size(5.dp).background(
                    when { !hasTask -> Color.Transparent; active -> kk.onPrimary; hasDue -> Accents.overdue().mark; else -> Accents.scheduled().text },
                    CircleShape))
            }
        }
    }
}
