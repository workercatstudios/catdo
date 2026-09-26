package com.workercat.catdo.ui

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.clickable
import androidx.compose.foundation.selection.toggleable
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.Logout
import androidx.compose.material.icons.automirrored.outlined.OpenInNew
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.sp
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.compose.LifecycleStartEffect
import com.clerk.api.Clerk
import com.clerk.ui.auth.AuthView
import com.google.firebase.messaging.FirebaseMessaging
import com.workercat.catdo.Diagnostics
import com.workercat.catdo.sync.TERMS_REVIEW_URL
import com.workercat.catdo.data.*
import androidx.core.content.ContextCompat
import kotlinx.coroutines.launch
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.time.LocalDate
import java.time.format.DateTimeFormatter

private val mainSections = listOf(Section.Today, Section.Inbox, Section.Upcoming, Section.Projects)

private fun Section.icon(): ImageVector = when (this) {
    Section.Today -> Icons.Outlined.WbSunny
    Section.Inbox -> Icons.Outlined.Inbox
    Section.Upcoming -> Icons.Outlined.DateRange
    Section.Calendar -> Icons.Outlined.CalendarMonth
    Section.Projects -> Icons.Outlined.FolderOpen
    Section.Completed -> Icons.Outlined.CheckCircle
    Section.Search -> Icons.Outlined.Search
    Section.Settings -> Icons.Outlined.Settings
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CatDoApp(vm: CatDoViewModel) {
    val state by vm.state.collectAsStateWithLifecycle()
    val data = state.data
    val workspace = data.workspaces.firstOrNull { it.id == vm.workspaceId && !it.archived }
        ?: data.workspaces.first { !it.archived }
    val drawer = rememberDrawerState(DrawerValue.Closed)
    val scope = rememberCoroutineScope()
    val snackbar = remember { SnackbarHostState() }
    val selectedProject = data.projects.firstOrNull { it.id == vm.projectId }
    val wide = with(LocalDensity.current) { LocalWindowInfo.current.containerSize.width.toDp() >= 600.dp }
    val context = LocalContext.current
    var notificationsEnabled by remember { mutableStateOf(false) }
    var notificationBusy by remember { mutableStateOf(true) }
    LaunchedEffect(Unit) {
        notificationsEnabled = runCatching {
            withContext(Dispatchers.IO) { FirebaseMessaging.getInstance().isAutoInitEnabled }
        }.getOrDefault(false)
        notificationBusy = false
    }
    val enableNotifications = {
        if (!notificationBusy) {
            notificationBusy = true
            scope.launch {
                try {
                    // Auto-init handles registration and retries. The switch reflects
                    // the saved preference, which can succeed before registration does.
                    notificationsEnabled = withContext(Dispatchers.IO) {
                        FirebaseMessaging.getInstance().apply { isAutoInitEnabled = true }.isAutoInitEnabled
                    }
                    if (notificationsEnabled) Diagnostics.event("notifications_enabled")
                    else vm.message = "Notifications could not be enabled. Please try again."
                } catch (error: Exception) {
                    Diagnostics.failure("notifications_enable_failed", error)
                    notificationsEnabled = runCatching {
                        withContext(Dispatchers.IO) { FirebaseMessaging.getInstance().isAutoInitEnabled }
                    }.getOrDefault(false)
                    if (!notificationsEnabled) vm.message = "Notifications could not be enabled. Please try again."
                } finally {
                    notificationBusy = false
                }
            }
        }
    }
    val notificationPermission = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        notificationBusy = false
        if (granted) enableNotifications()
        else vm.message = "Allow notifications in Android settings to receive CatDo alerts."
    }
    val toggleNotifications: (Boolean) -> Unit = { enabled ->
        if (notificationBusy) Unit
        else if (!enabled) {
            notificationBusy = true
            scope.launch {
                try {
                    val cleanup = withContext(Dispatchers.IO) {
                        FirebaseMessaging.getInstance().run {
                            isAutoInitEnabled = false
                            unregister()
                        }
                    }
                    notificationsEnabled = false
                    cleanup.addOnFailureListener { error ->
                        Diagnostics.failure("notifications_unregister_failed", error)
                    }
                } catch (error: Exception) {
                    Diagnostics.failure("notifications_unregister_failed", error)
                    notificationsEnabled = runCatching {
                        withContext(Dispatchers.IO) { FirebaseMessaging.getInstance().isAutoInitEnabled }
                    }.getOrDefault(true)
                    if (notificationsEnabled) vm.message = "Could not turn off notifications. Please try again."
                } finally {
                    notificationBusy = false
                }
            }
        } else if (Build.VERSION.SDK_INT >= 33 &&
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            notificationBusy = true
            try { notificationPermission.launch(Manifest.permission.POST_NOTIFICATIONS) }
            catch (error: Exception) {
                Diagnostics.failure("notifications_permission_failed", error)
                notificationBusy = false
                vm.message = "Could not ask for notification permission. Try again."
            }
        }
        else enableNotifications()
    }

    LifecycleStartEffect(vm.signedIn) {
        val poll = if (vm.signedIn) scope.launch {
            vm.sync()
            while (isActive) { delay(30_000); vm.sync() }
        } else null
        onStopOrDispose { poll?.cancel() }
    }

    LaunchedEffect(vm.message) {
        vm.message?.let { snackbar.showSnackbar(it); vm.message = null }
    }
    LaunchedEffect(vm.undoPrompt) {
        vm.undoPrompt?.let { prompt ->
            vm.undoPrompt = null
            if (snackbar.showSnackbar(prompt, actionLabel = "Undo", duration = SnackbarDuration.Short) == SnackbarResult.ActionPerformed) vm.undo()
        }
    }

    if (vm.eligibilityOpen) {
        var ageConfirmed by remember { mutableStateOf(false) }
        val uriHandler = LocalUriHandler.current
        AlertDialog(
            onDismissRequest = vm::closeAuth,
            title = { Text("Before you sign in") },
            text = {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                    Row(verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier.fillMaxWidth().toggleable(value = ageConfirmed, role = Role.Checkbox,
                            onValueChange = { ageConfirmed = it })) {
                        Checkbox(checked = ageConfirmed, onCheckedChange = null)
                        Text("I am 13 or older")
                    }
                    Text("By checking this, I also confirm I meet any higher local minimum age and have guardian permission where required.")
                    Text("You will review and accept the WorkerCat terms before syncing. Local tasks remain available without an account.",
                        modifier = Modifier.padding(top = 12.dp))
                    Row {
                        for ((label, path) in listOf("Terms" to "terms", "Privacy" to "privacy")) {
                            TextButton(onClick = {
                                runCatching { uriHandler.openUri("https://workercat.com/$path") }
                                    .onFailure { vm.message = "Open https://workercat.com/$path in your browser." }
                            }) { Text(label) }
                        }
                    }
                }
            },
            confirmButton = {
                TextButton(enabled = ageConfirmed, onClick = { vm.confirmEligibility(ageConfirmed) }) { Text("Continue to sign in") }
            },
            dismissButton = { TextButton(onClick = vm::closeAuth) { Text("Keep using locally") } },
        )
    }

    if (vm.authOpen) {
        val ready by vm.clerkReady.collectAsStateWithLifecycle()
        val error by vm.clerkError.collectAsStateWithLifecycle()
        when {
            ready -> AuthView(
                onDismiss = vm::closeAuth,
                onAuthComplete = vm::authComplete,
            )
            else -> Column(Modifier.fillMaxSize().padding(32.dp),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally) {
                if (error == null) {
                    CircularProgressIndicator()
                    Spacer(Modifier.height(20.dp))
                    Text("Connecting to sign-in…")
                } else {
                    Text("Can't reach sign-in", style = MaterialTheme.typography.headlineSmall)
                    Spacer(Modifier.height(8.dp))
                    Text("Check your connection or Private DNS, then try again.")
                    Spacer(Modifier.height(20.dp))
                    Button(onClick = { Clerk.reinitialize() }) { Text("Try again") }
                }
                TextButton(onClick = vm::closeAuth) { Text("Back to CatDo") }
            }
        }
        return
    }

    ModalNavigationDrawer(
        drawerState = drawer,
        drawerContent = {
            ModalDrawerSheet(modifier = Modifier.width(310.dp), drawerContainerColor = MaterialTheme.colorScheme.surfaceContainerLow) {
                Column(Modifier.weight(1f).verticalScroll(rememberScrollState())) {
                    Spacer(Modifier.height(12.dp))
                    Text("CatDo", style = MaterialTheme.typography.titleLarge, color = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.padding(horizontal = 28.dp), fontWeight = FontWeight.Bold)
                    Row(Modifier.fillMaxWidth().clickable { vm.nameDialog = "switch" }.padding(horizontal = 28.dp).height(56.dp),
                        verticalAlignment = Alignment.CenterVertically) {
                        Text(workspace.name, Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.Medium)
                        Icon(Icons.Outlined.KeyboardArrowDown, "Switch workspace", Modifier.size(18.dp))
                    }
                    Spacer(Modifier.height(8.dp))
                    listOf(Section.Today, Section.Inbox, Section.Upcoming, Section.Calendar, Section.Completed, Section.Search).forEach { section ->
                        NavigationDrawerItem(
                            label = { Text(section.name, style = MaterialTheme.typography.bodyMedium) }, icon = { Icon(section.icon(), null, Modifier.size(20.dp)) },
                            selected = vm.section == section,
                            onClick = { vm.select(section); scope.launch { drawer.close() } },
                            shape = MaterialTheme.shapes.small,
                            modifier = Modifier.padding(horizontal = 12.dp, vertical = 1.dp).height(48.dp),
                        )
                    }
                    HorizontalDivider(Modifier.padding(horizontal = 20.dp, vertical = 12.dp))
                    Row(Modifier.fillMaxWidth().padding(start = 28.dp, end = 16.dp), verticalAlignment = Alignment.CenterVertically) {
                        Text("Projects", Modifier.weight(1f), style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant, fontWeight = FontWeight.Bold)
                        IconButton(onClick = { vm.nameDialog = "project"; scope.launch { drawer.close() } }) {
                            Icon(Icons.Default.Add, "New project")
                        }
                    }
                    data.projects.filter { it.workspaceId == workspace.id && !it.archived }.forEach { project ->
                        NavigationDrawerItem(
                            label = { Text(project.name, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                            icon = { Icon(Icons.Outlined.FolderOpen, null, Modifier.size(20.dp)) },
                            selected = vm.section == Section.Projects && vm.projectId == project.id,
                            onClick = { vm.select(Section.Projects, project.id); scope.launch { drawer.close() } },
                            shape = MaterialTheme.shapes.small,
                            modifier = Modifier.padding(horizontal = 12.dp, vertical = 1.dp).height(48.dp),
                        )
                    }
                    Spacer(Modifier.height(16.dp))
                    NavigationDrawerItem(
                        label = { Text("Settings") }, icon = { Icon(Icons.Outlined.Settings, null) },
                        selected = vm.section == Section.Settings,
                        onClick = { vm.select(Section.Settings); scope.launch { drawer.close() } },
                        shape = MaterialTheme.shapes.small,
                        modifier = Modifier.padding(horizontal = 12.dp, vertical = 1.dp).height(48.dp),
                    )
                    Spacer(Modifier.height(20.dp))
                }
            }
        },
    ) {
        Scaffold(
            containerColor = MaterialTheme.colorScheme.background,
            topBar = {
                TopAppBar(
                    expandedHeight = 48.dp,
                    title = {
                        Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.clickable { vm.nameDialog = "switch" }) {
                            Text(workspace.name, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold,
                                maxLines = 1, overflow = TextOverflow.Ellipsis)
                            Icon(Icons.Outlined.KeyboardArrowDown, "Switch workspace", Modifier.size(22.dp))
                        }
                    },
                    navigationIcon = { IconButton(onClick = { scope.launch { drawer.open() } }) { Icon(Icons.Outlined.Menu, "Open menu") } },
                    actions = {
                        IconButton(onClick = { vm.select(Section.Search) }) { Icon(Icons.Outlined.Search, "Search tasks") }
                    },
                    colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background),
                )
            },
            bottomBar = {
                if (!wide) NavigationBar(containerColor = MaterialTheme.colorScheme.surfaceContainerLow, tonalElevation = 0.dp) {
                    mainSections.forEach { section ->
                        NavigationBarItem(
                            selected = vm.section == section && (section != Section.Projects || vm.projectId == null),
                            onClick = { vm.select(section) },
                            icon = { Icon(section.icon(), null) }, label = { Text(section.name) },
                            alwaysShowLabel = true,
                        )
                    }
                }
            },
            snackbarHost = { SnackbarHost(snackbar) },
        ) { padding ->
            Row(Modifier.fillMaxSize().padding(padding)) {
                if (wide) NavigationRail(containerColor = MaterialTheme.colorScheme.surfaceContainerLow) {
                    mainSections.forEach { section ->
                        NavigationRailItem(
                            selected = vm.section == section && (section != Section.Projects || vm.projectId == null),
                            onClick = { vm.select(section) },
                            icon = { Icon(section.icon(), null) }, label = { Text(section.name) },
                        )
                    }
                }
                Box(Modifier.weight(1f).fillMaxHeight(), contentAlignment = Alignment.TopCenter) {
                Column(Modifier.widthIn(max = 840.dp).fillMaxSize()) {
                when (vm.section) {
                    Section.Projects -> if (vm.projectId == null) ProjectOverview(data, workspace, vm) else
                        TaskSection(data, vm, selectedProject?.name ?: "Project",
                            data.tasks.filter { it.workspaceId == workspace.id && it.projectId == vm.projectId && it.completedAt == null })
                    Section.Settings -> SettingsScreen(workspace, vm, notificationsEnabled, notificationBusy, toggleNotifications)
                    Section.Search -> SearchScreen(data, workspace, vm)
                    Section.Calendar -> CalendarScreen(data, workspace, vm)
                    else -> {
                        val day = today()
                        val tasks = data.tasks.filter { it.workspaceId == workspace.id && it.isActive(data) }
                        val shown = when (vm.section) {
                            Section.Today -> tasks.filter { it.isToday(day) }
                            Section.Inbox -> tasks.filter { it.projectId == null && it.scheduled == null && it.due == null }
                            Section.Upcoming -> tasks.filter { (it.scheduled ?: it.due)?.let { date -> date > day } == true || (it.due?.let { date -> date > day } == true) }
                            Section.Completed -> data.tasks.filter { it.workspaceId == workspace.id && it.completedAt != null }
                            else -> emptyList()
                        }
                        val title = vm.section.name
                        TaskSection(data, vm, title, shown)
                    }
                }
                }
                }
            }
        }
    }

    vm.editor?.let { task ->
        TaskEditorSheet(
            task = task, data = data, onDismiss = { vm.editor = null },
            onSave = { vm.save(it); vm.editor = null },
            onDelete = { vm.delete(task.id); vm.editor = null },
            onAddSubtask = { vm.addSubtask(it) },
            onEditSubtask = { vm.edit(it) },
            onCompleteSubtask = { vm.complete(it.id) },
            isNew = vm.creating,
        )
    }

    vm.nameDialog?.let { type ->
        when (type) {
            "switch" -> AlertDialog(
                onDismissRequest = { vm.nameDialog = null }, title = { Text("Workspaces") },
                text = { Column {
                    data.workspaces.filter { !it.archived }.forEach { item ->
                        TextButton(onClick = { vm.workspaceId = item.id; vm.select(Section.Today); vm.nameDialog = null },
                            modifier = Modifier.fillMaxWidth()) { Text(item.name, Modifier.fillMaxWidth()) }
                    }
                    TextButton(onClick = { vm.nameDialog = "workspace" }) { Icon(Icons.Default.Add, null); Spacer(Modifier.width(8.dp)); Text("New workspace") }
                } },
                confirmButton = { TextButton(onClick = { vm.nameDialog = null }) { Text("Done") } },
            )
            else -> NameDialog(if (type == "workspace") "New workspace" else "New project",
                onDismiss = { vm.nameDialog = null }, onCreate = {
                    if (type == "workspace") vm.createWorkspace(it) else vm.createProject(it)
                    vm.nameDialog = null
                })
        }
    }

    state.conflict?.let {
        AlertDialog(
            onDismissRequest = {}, title = { Text("Choose which changes to keep") },
            text = { Text("Tasks were changed on this device and another device. Your choice applies to the conflicting records. Other changes are combined.") },
            confirmButton = { TextButton(onClick = { vm.resolveConflict(false) }) { Text("Keep this device") } },
            dismissButton = { TextButton(onClick = { vm.resolveConflict(true) }) { Text("Keep cloud changes") } },
        )
    }
}

@Composable
private fun NameDialog(title: String, onDismiss: () -> Unit, onCreate: (String) -> Unit) {
    var name by remember(title) { mutableStateOf("") }
    AlertDialog(
        onDismissRequest = onDismiss, title = { Text(title) },
        text = { OutlinedTextField(name, { name = it }, label = { Text("Name") }, singleLine = true) },
        confirmButton = { TextButton(onClick = { onCreate(name) }, enabled = name.trim().isNotBlank()) { Text("Create") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
internal fun SectionHeading(title: String, count: Int = 0, detail: String? = null, onAdd: (() -> Unit)? = null) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(top = 16.dp, bottom = 20.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Row(Modifier.weight(1f), verticalAlignment = Alignment.CenterVertically) {
                Text(title, style = MaterialTheme.typography.headlineLarge, modifier = Modifier.weight(1f, fill = false))
                if (count > 0) Text(count.toString(), modifier = Modifier.padding(start = 10.dp),
                    style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            if (onAdd != null) Button(onClick = onAdd, shape = MaterialTheme.shapes.small,
                contentPadding = PaddingValues(horizontal = 12.dp), modifier = Modifier.height(36.dp)) {
                Icon(Icons.Outlined.Add, null, Modifier.size(16.dp))
                Spacer(Modifier.width(6.dp))
                Text("Add task", style = MaterialTheme.typography.labelMedium)
            }
        }
        if (detail != null) Text(detail, style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(top = 4.dp))
    }
}

@Composable
private fun TaskSection(data: AppData, vm: CatDoViewModel, title: String, tasks: List<Task>) {
    val sorted = tasks.sortedWith(compareBy<Task> { it.scheduled ?: it.due ?: "9999-12-31" }.thenBy { it.createdAt })
    val isToday = vm.section == Section.Today
    val overdue = if (isToday) sorted.filter { it.due != null && it.due < today() } else emptyList()
    val remaining = sorted.filterNot { it in overdue }
    LazyColumn(contentPadding = PaddingValues(bottom = 24.dp)) {
        item {
            SectionHeading(title, sorted.size,
                detail = if (isToday) LocalDate.now().format(DateTimeFormatter.ofPattern("EEEE, MMMM d")) else null,
                onAdd = { vm.newTask(if (isToday) today() else null) })
        }
        if (sorted.isEmpty()) item { EmptyState(if (isToday) "You’re all caught up" else "No tasks yet", "") }
        if (overdue.isNotEmpty()) {
            item { TaskGroupHeading("Overdue", overdue.size, overdue = true) }
            items(overdue, key = { it.id }) { task ->
                TaskRow(task, data, { vm.edit(task) }, { vm.complete(task.id) }, showScheduled = false)
            }
            if (remaining.isNotEmpty()) item { TaskGroupHeading("Today", remaining.size) }
        }
        items(remaining, key = { it.id }) { task ->
            TaskRow(task, data, { vm.edit(task) }, { vm.complete(task.id) }, showScheduled = !isToday)
        }
    }
}

@Composable
private fun TaskGroupHeading(title: String, count: Int, overdue: Boolean = false) {
    Column {
        Row(Modifier.fillMaxWidth().padding(start = 20.dp, end = 20.dp, top = 12.dp, bottom = 8.dp),
            verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(title, style = MaterialTheme.typography.labelMedium,
                color = if (overdue) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface)
            Text(count.toString(), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        HorizontalDivider(Modifier.padding(horizontal = 20.dp))
    }
}

@Composable
fun TaskRow(task: Task, data: AppData, onOpen: () -> Unit, onComplete: () -> Unit, showScheduled: Boolean = true) {
    val completed = task.completedAt != null
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    val overdue = task.due != null && task.due < today() && !completed
    val metadata = buildAnnotatedString {
        task.due?.let { due ->
            withStyle(SpanStyle(color = if (overdue) MaterialTheme.colorScheme.error else muted)) { append("Due ${formatDay(due)}") }
        }
        val details = buildList {
            if (showScheduled) task.scheduled?.let { add(formatDay(it)) }
            task.projectId?.let { id -> data.projects.firstOrNull { it.id == id }?.name?.let { add(it) } }
            task.recurrence?.let { add("Repeats") }
            data.tasks.count { it.parentId == task.id }.takeIf { it > 0 }?.let { add("$it subtasks") }
        }
        if (details.isNotEmpty()) {
            if (length > 0) append(" · ")
            append(details.joinToString(" · "))
        }
    }
    Column(Modifier.fillMaxWidth()) {
        Row(Modifier.fillMaxWidth().heightIn(min = 64.dp).clickable(onClick = onOpen).padding(start = 8.dp, end = 20.dp),
            verticalAlignment = Alignment.Top) {
            IconButton(onClick = onComplete, modifier = Modifier.size(48.dp).padding(top = 3.dp)) {
                Icon(if (completed) Icons.Outlined.CheckCircle else Icons.Outlined.RadioButtonUnchecked,
                    if (completed) "Restore ${task.title}" else "Complete ${task.title}", modifier = Modifier.size(20.dp),
                    tint = if (completed) MaterialTheme.colorScheme.primary else muted.copy(alpha = 0.7f))
            }
            Column(Modifier.weight(1f).padding(top = 13.dp, bottom = 12.dp)) {
                Text(task.title, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Normal,
                    color = if (completed) muted else MaterialTheme.colorScheme.onSurface,
                    textDecoration = if (completed) TextDecoration.LineThrough else TextDecoration.None)
                if (task.notes.isNotBlank()) Text(task.notes, maxLines = 1, overflow = TextOverflow.Ellipsis,
                    style = MaterialTheme.typography.bodySmall, color = muted, modifier = Modifier.padding(top = 2.dp))
                if (metadata.isNotEmpty()) Text(metadata, fontSize = 11.sp, lineHeight = 16.sp, maxLines = 1,
                    overflow = TextOverflow.Ellipsis, color = muted, modifier = Modifier.padding(top = 3.dp))
            }
        }
    }
}

fun formatDay(day: String): String = try {
    val date = LocalDate.parse(day)
    when (date) {
        LocalDate.now() -> "Today"
        LocalDate.now().plusDays(1) -> "Tomorrow"
        else -> date.format(DateTimeFormatter.ofPattern("MMM d"))
    }
} catch (_: Exception) { day }

@Composable
private fun EmptyState(title: String, subtitle: String) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 32.dp)) {
        Text(title, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
        if (subtitle.isNotBlank()) Text(subtitle, style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(top = 4.dp))
    }
}

@Composable
private fun ProjectOverview(data: AppData, workspace: Workspace, vm: CatDoViewModel) {
    val projects = data.projects.filter { it.workspaceId == workspace.id && !it.archived }
    LazyColumn(contentPadding = PaddingValues(bottom = 24.dp)) {
        item { SectionHeading("Projects", projects.size) }
        if (projects.isEmpty()) item { EmptyState("No projects yet", "") }
        items(projects, key = { it.id }) { project ->
            val count = data.tasks.count { it.projectId == project.id && it.isActive(data) }
            Row(Modifier.fillMaxWidth().clickable { vm.select(Section.Projects, project.id) }
                .padding(horizontal = 20.dp).heightIn(min = 60.dp), verticalAlignment = Alignment.CenterVertically) {
                Icon(Icons.Outlined.FolderOpen, null, Modifier.size(20.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(project.name, Modifier.weight(1f).padding(horizontal = 12.dp), style = MaterialTheme.typography.bodyLarge)
                Text(count.toString(), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Icon(Icons.Outlined.ChevronRight, null, Modifier.padding(start = 12.dp).size(18.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            HorizontalDivider(Modifier.padding(horizontal = 20.dp))
        }
        item { TextButton(onClick = { vm.nameDialog = "project" }, modifier = Modifier.padding(start = 12.dp, top = 8.dp)) {
            Icon(Icons.Default.Add, null, Modifier.size(18.dp)); Spacer(Modifier.width(8.dp)); Text("New project")
        } }
    }
}

@Composable
private fun SearchScreen(data: AppData, workspace: Workspace, vm: CatDoViewModel) {
    val query = vm.search.trim()
    val tasks = data.tasks.filter { it.workspaceId == workspace.id && it.completedAt == null &&
        query.isNotEmpty() && (it.title.contains(query, true) || it.notes.contains(query, true)) }
    Column {
        SectionHeading("Search", tasks.size)
        OutlinedTextField(vm.search, { vm.search = it }, modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp),
            placeholder = { Text("Search tasks") }, leadingIcon = { Icon(Icons.Outlined.Search, null) }, singleLine = true, shape = MaterialTheme.shapes.small)
        Spacer(Modifier.height(16.dp))
        LazyColumn(contentPadding = PaddingValues(bottom = 100.dp)) {
            items(tasks, key = { it.id }) { task -> TaskRow(task, data, { vm.edit(task) }, { vm.complete(task.id) }) }
            if (query.isNotEmpty() && tasks.isEmpty()) item { EmptyState("No matching tasks.", "Try a different word.") }
        }
    }
}

@Composable
private fun SettingsScreen(workspace: Workspace, vm: CatDoViewModel, notificationsEnabled: Boolean, notificationBusy: Boolean, onNotificationsChanged: (Boolean) -> Unit) {
    val uriHandler = LocalUriHandler.current
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(bottom = 32.dp)) {
        SectionHeading("Settings")
        SettingsGroup("Workspace") {
            SettingsRow(workspace.name, "", Icons.Outlined.Workspaces,
                onClick = { vm.nameDialog = "switch" })
            HorizontalDivider(Modifier.padding(start = 56.dp, end = 16.dp))
            SettingsRow("New workspace", "", Icons.Outlined.AddCircleOutline,
                onClick = { vm.nameDialog = "workspace" })
        }
        SettingsGroup("This device") {
            SettingsRow("Available offline", "", Icons.Outlined.OfflinePin)
            HorizontalDivider(Modifier.padding(start = 56.dp, end = 16.dp))
            SettingsRow("Notifications", "", Icons.Outlined.NotificationsNone,
                trailing = {
                    Switch(checked = notificationsEnabled, enabled = !notificationBusy, onCheckedChange = onNotificationsChanged)
                })
        }
        SettingsGroup("Account & sync") {
            if (vm.signedIn) {
                if (vm.syncTermsRequired) {
                    SettingsRow("Review terms in browser", "Use the same CatDo account, then tap Sync now", Icons.AutoMirrored.Outlined.OpenInNew,
                        onClick = {
                            runCatching { uriHandler.openUri(TERMS_REVIEW_URL) }
                                .onFailure { vm.message = "Open $TERMS_REVIEW_URL in your browser to review the terms." }
                        })
                    HorizontalDivider(Modifier.padding(start = 56.dp, end = 16.dp))
                }
                SettingsRow(if (vm.syncing) "Syncing…" else "Sync now",
                    vm.syncStatus ?: "Keep your WorkerCat devices up to date", Icons.Outlined.Sync,
                    enabled = !vm.syncing, onClick = { vm.sync() })
                HorizontalDivider(Modifier.padding(start = 56.dp, end = 16.dp))
                SettingsRow("Sign out", "Tasks remain on this device", Icons.AutoMirrored.Outlined.Logout,
                    onClick = { vm.signOut() })
            } else {
                SettingsRow(if (vm.syncing) "Connecting…" else "Sign in to sync",
                    "Use your WorkerCat account across devices", Icons.Outlined.CloudSync,
                    enabled = !vm.syncing, onClick = { vm.startLogin() })
            }
        }

    }
}

@Composable
private fun SettingsGroup(title: String, content: @Composable ColumnScope.() -> Unit) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 20.dp)) {
        Text(title, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(bottom = 4.dp))
        Column(content = content)
    }
}

@Composable
private fun SettingsRow(title: String, description: String, icon: ImageVector, enabled: Boolean = true,
    onClick: (() -> Unit)? = null, trailing: (@Composable () -> Unit)? = null) {
    ListItem(
        headlineContent = { Text(title, style = MaterialTheme.typography.bodyLarge) },
        supportingContent = if (description.isBlank()) null else { { Text(description, style = MaterialTheme.typography.bodySmall) } },
        leadingContent = { Icon(icon, null, Modifier.size(20.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant) },
        trailingContent = trailing ?: onClick?.let { { Icon(Icons.Outlined.ChevronRight, null, Modifier.size(18.dp)) } },
        colors = ListItemDefaults.colors(containerColor = MaterialTheme.colorScheme.background),
        modifier = if (onClick != null) Modifier.clickable(enabled = enabled, onClick = onClick) else Modifier,
    )
}
