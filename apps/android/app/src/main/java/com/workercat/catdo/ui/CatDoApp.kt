package com.workercat.catdo.ui

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Intent
import androidx.core.net.toUri
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.Logout
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.compose.LifecycleStartEffect
import com.workercat.catdo.data.*
import kotlinx.coroutines.launch
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle

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
    val context = LocalContext.current
    val wide = with(LocalDensity.current) { LocalWindowInfo.current.containerSize.width.toDp() >= 600.dp }

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

    ModalNavigationDrawer(
        drawerState = drawer,
        drawerContent = {
            ModalDrawerSheet(modifier = Modifier.width(310.dp)) {
                Spacer(Modifier.height(24.dp))
                Text("CATDO", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.padding(horizontal = 28.dp), fontWeight = FontWeight.Bold)
                Spacer(Modifier.height(16.dp))
                Text(workspace.name, style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.SemiBold,
                    modifier = Modifier.padding(horizontal = 28.dp))
                Spacer(Modifier.height(16.dp))
                listOf(Section.Today, Section.Inbox, Section.Upcoming, Section.Calendar, Section.Completed, Section.Search).forEach { section ->
                    NavigationDrawerItem(
                        label = { Text(section.name) }, icon = { Icon(section.icon(), null) },
                        selected = vm.section == section,
                        onClick = { vm.select(section); scope.launch { drawer.close() } },
                        modifier = Modifier.padding(horizontal = 12.dp),
                    )
                }
                HorizontalDivider(Modifier.padding(20.dp))
                Row(Modifier.fillMaxWidth().padding(start = 28.dp, end = 16.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text("PROJECTS", Modifier.weight(1f), style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant, fontWeight = FontWeight.Bold)
                    IconButton(onClick = { vm.nameDialog = "project"; scope.launch { drawer.close() } }) {
                        Icon(Icons.Default.Add, "New project")
                    }
                }
                data.projects.filter { it.workspaceId == workspace.id && !it.archived }.forEach { project ->
                    NavigationDrawerItem(
                        label = { Text(project.name, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                        icon = { Icon(Icons.Outlined.Folder, null) },
                        selected = vm.section == Section.Projects && vm.projectId == project.id,
                        onClick = { vm.select(Section.Projects, project.id); scope.launch { drawer.close() } },
                        modifier = Modifier.padding(horizontal = 12.dp),
                    )
                }
                Spacer(Modifier.weight(1f))
                NavigationDrawerItem(
                    label = { Text("Settings") }, icon = { Icon(Icons.Outlined.Settings, null) },
                    selected = vm.section == Section.Settings,
                    onClick = { vm.select(Section.Settings); scope.launch { drawer.close() } },
                    modifier = Modifier.padding(horizontal = 12.dp),
                )
                Spacer(Modifier.height(20.dp))
            }
        },
    ) {
        Scaffold(
            containerColor = MaterialTheme.colorScheme.background,
            topBar = {
                TopAppBar(
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
                        IconButton(onClick = { vm.select(Section.Settings) }) { Icon(Icons.Outlined.MoreVert, "Settings") }
                    },
                    colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background),
                )
            },
            bottomBar = {
                if (!wide) NavigationBar(containerColor = MaterialTheme.colorScheme.surface) {
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
            floatingActionButton = {
                if (vm.section != Section.Settings) {
                    ExtendedFloatingActionButton(
                        onClick = { vm.newTask(if (vm.section == Section.Today) today() else null) },
                        icon = { Icon(Icons.Default.Add, null) }, text = { Text("Add task") },
                        containerColor = MaterialTheme.colorScheme.primary,
                        contentColor = MaterialTheme.colorScheme.onPrimary,
                    )
                }
            },
            snackbarHost = { SnackbarHost(snackbar) },
        ) { padding ->
            Row(Modifier.fillMaxSize().padding(padding)) {
                if (wide) NavigationRail(containerColor = MaterialTheme.colorScheme.surface) {
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
                        TaskSection(data, workspace, vm, selectedProject?.name ?: "Project", "Keep the next step moving.",
                            data.tasks.filter { it.workspaceId == workspace.id && it.projectId == vm.projectId && it.completedAt == null })
                    Section.Settings -> SettingsScreen(workspace, vm)
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
                        val subtitle = when (vm.section) {
                            Section.Today -> "A clear place to start."
                            Section.Inbox -> "Catch it here. Sort it later."
                            Section.Upcoming -> "A little room to look ahead."
                            Section.Completed -> "The things you made time for."
                            else -> ""
                        }
                        TaskSection(data, workspace, vm, title, subtitle, shown)
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

    vm.deviceLogin?.let { login ->
        AlertDialog(
            onDismissRequest = { vm.deviceLogin = null },
            title = { Text("Sign in to sync") },
            text = { Column {
                Text("Open the secure sign-in page and enter this code:")
                Spacer(Modifier.height(18.dp))
                Text(login.userCode, style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)
                Spacer(Modifier.height(10.dp))
                Text("This screen will close when you finish signing in.", style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
            } },
            confirmButton = { TextButton(onClick = {
                val url = login.verificationUriComplete ?: login.verificationUri
                context.startActivity(Intent(Intent.ACTION_VIEW, url.toUri()))
            }) { Text("Open sign-in") } },
            dismissButton = { TextButton(onClick = {
                val clipboard = context.getSystemService(ClipboardManager::class.java)
                clipboard.setPrimaryClip(ClipData.newPlainText("CatDo sign-in code", login.userCode))
            }) { Text("Copy code") } },
        )
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
private fun SectionHeading(eyebrow: String, title: String, subtitle: String, count: Int) {
    Column(Modifier.fillMaxWidth().padding(start = 24.dp, end = 24.dp, top = 28.dp, bottom = 16.dp)) {
        Text(eyebrow.uppercase(), style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.primary, fontWeight = FontWeight.Bold)
        Spacer(Modifier.height(12.dp))
        Text(title, style = MaterialTheme.typography.headlineLarge, fontWeight = FontWeight.Bold)
        Spacer(Modifier.height(6.dp))
        Text(subtitle, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        if (count > 0) {
            Spacer(Modifier.height(24.dp))
            Text("$count ${if (count == 1) "task" else "tasks"}", style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

@Composable
private fun TaskSection(data: AppData, workspace: Workspace, vm: CatDoViewModel, title: String, subtitle: String, tasks: List<Task>) {
    val sorted = tasks.sortedWith(compareBy<Task> { it.scheduled ?: it.due ?: "9999-12-31" }.thenBy { it.createdAt })
    LazyColumn(contentPadding = PaddingValues(bottom = 100.dp)) {
        item {
            val date = if (vm.section == Section.Today) LocalDate.now().format(DateTimeFormatter.ofLocalizedDate(FormatStyle.FULL)) else workspace.name
            SectionHeading(date, title, subtitle, sorted.size)
        }
        if (sorted.isEmpty()) item {
            EmptyState(
                if (vm.section == Section.Today) "A little breathing room." else "Nothing here yet.",
                if (vm.section == Section.Today) "Add a task when you know what comes next." else "Your tasks will show up here as you go.",
            )
        }
        items(sorted, key = { it.id }) { task -> TaskRow(task, data, onOpen = { vm.edit(task) }, onComplete = { vm.complete(task.id) }) }
    }
}

@Composable
fun TaskRow(task: Task, data: AppData, onOpen: () -> Unit, onComplete: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().clickable(onClick = onOpen).padding(horizontal = 22.dp, vertical = 5.dp),
        verticalAlignment = Alignment.Top,
    ) {
        IconButton(onClick = onComplete, modifier = Modifier.size(42.dp)) {
            Icon(if (task.completedAt == null) Icons.Outlined.RadioButtonUnchecked else Icons.Outlined.CheckCircle,
                if (task.completedAt == null) "Complete ${task.title}" else "Restore ${task.title}",
                tint = if (task.completedAt == null) MaterialTheme.colorScheme.outline else MaterialTheme.colorScheme.primary)
        }
        Column(Modifier.weight(1f).padding(start = 8.dp, top = 7.dp, bottom = 16.dp)) {
            Text(task.title, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
            if (task.notes.isNotBlank()) {
                Spacer(Modifier.height(4.dp))
                Text(task.notes, maxLines = 1, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            val details = buildList {
                task.scheduled?.let { add("Plan ${formatDay(it)}") }
                task.due?.let { add("Due ${formatDay(it)}") }
                task.projectId?.let { id -> data.projects.firstOrNull { it.id == id }?.name?.let { add(it) } }
                task.recurrence?.let { add("Repeats") }
                data.tasks.count { it.parentId == task.id }.takeIf { it > 0 }?.let { add("$it subtasks") }
            }
            if (details.isNotEmpty()) {
                Spacer(Modifier.height(7.dp))
                Text(details.joinToString("  ·  "), style = MaterialTheme.typography.labelMedium,
                    color = if (task.due != null && task.due < today() && task.completedAt == null) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Spacer(Modifier.height(15.dp))
            HorizontalDivider(color = MaterialTheme.colorScheme.outline.copy(alpha = .5f))
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
    Column(Modifier.fillMaxWidth().padding(horizontal = 32.dp, vertical = 70.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        Surface(shape = MaterialTheme.shapes.extraLarge, color = MaterialTheme.colorScheme.primaryContainer,
            modifier = Modifier.size(76.dp)) {
            Box(contentAlignment = Alignment.Center) { Icon(Icons.Outlined.CheckCircleOutline, null,
                Modifier.size(34.dp), tint = MaterialTheme.colorScheme.primary) }
        }
        Spacer(Modifier.height(22.dp))
        Text(title, style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
        Spacer(Modifier.height(6.dp))
        Text(subtitle, style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
private fun ProjectOverview(data: AppData, workspace: Workspace, vm: CatDoViewModel) {
    val projects = data.projects.filter { it.workspaceId == workspace.id && !it.archived }
    LazyColumn(contentPadding = PaddingValues(bottom = 100.dp)) {
        item { SectionHeading(workspace.name, "Projects", "A place for the bigger things.", projects.size) }
        if (projects.isEmpty()) item { EmptyState("Start something good.", "Create a project to bring related tasks together.") }
        items(projects, key = { it.id }) { project ->
            val count = data.tasks.count { it.projectId == project.id && it.isActive(data) }
            Surface(
                modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 6.dp).clickable { vm.select(Section.Projects, project.id) },
                shape = MaterialTheme.shapes.large, color = MaterialTheme.colorScheme.surface,
                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = .65f)),
            ) {
                Row(Modifier.padding(20.dp), verticalAlignment = Alignment.CenterVertically) {
                    Icon(Icons.Outlined.FolderOpen, null, tint = MaterialTheme.colorScheme.primary)
                    Column(Modifier.weight(1f).padding(start = 16.dp)) {
                        Text(project.name, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                        Text("$count ${if (count == 1) "open task" else "open tasks"}", style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    Icon(Icons.Outlined.ChevronRight, null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }
        item { TextButton(onClick = { vm.nameDialog = "project" }, modifier = Modifier.padding(start = 24.dp, top = 12.dp)) {
            Icon(Icons.Default.Add, null); Spacer(Modifier.width(8.dp)); Text("New project")
        } }
    }
}

@Composable
private fun SearchScreen(data: AppData, workspace: Workspace, vm: CatDoViewModel) {
    val query = vm.search.trim()
    val tasks = data.tasks.filter { it.workspaceId == workspace.id && it.completedAt == null &&
        query.isNotEmpty() && (it.title.contains(query, true) || it.notes.contains(query, true)) }
    Column {
        SectionHeading(workspace.name, "Search", "Find what you need.", tasks.size)
        OutlinedTextField(vm.search, { vm.search = it }, modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp),
            placeholder = { Text("Search tasks") }, leadingIcon = { Icon(Icons.Outlined.Search, null) }, singleLine = true)
        Spacer(Modifier.height(16.dp))
        LazyColumn(contentPadding = PaddingValues(bottom = 100.dp)) {
            items(tasks, key = { it.id }) { task -> TaskRow(task, data, { vm.edit(task) }, { vm.complete(task.id) }) }
            if (query.isNotEmpty() && tasks.isEmpty()) item { EmptyState("No matching tasks.", "Try a different word.") }
        }
    }
}

@Composable
private fun SettingsScreen(workspace: Workspace, vm: CatDoViewModel) {
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(bottom = 100.dp)) {
        SectionHeading(workspace.name, "Settings", "Make CatDo yours.", 0)
        ListItem(headlineContent = { Text("Workspace") }, supportingContent = { Text(workspace.name) },
            leadingContent = { Icon(Icons.Outlined.Workspaces, null) },
            trailingContent = { Icon(Icons.Outlined.ChevronRight, null) },
            modifier = Modifier.clickable { vm.nameDialog = "switch" })
        ListItem(headlineContent = { Text("New workspace") }, supportingContent = { Text("Keep work and personal plans apart") },
            leadingContent = { Icon(Icons.Outlined.AddCircleOutline, null) }, modifier = Modifier.clickable { vm.nameDialog = "workspace" })
        HorizontalDivider(Modifier.padding(horizontal = 24.dp, vertical = 14.dp))
        ListItem(headlineContent = { Text("Saved on this device") },
            supportingContent = { Text("Your tasks stay available without a connection.") },
            leadingContent = { Icon(Icons.Outlined.OfflinePin, null, tint = MaterialTheme.colorScheme.primary) })
        if (vm.signedIn) {
            ListItem(headlineContent = { Text(if (vm.syncing) "Syncing…" else "Sync now") },
                supportingContent = { Text("Keep this device up to date with your WorkerCat account") },
                leadingContent = { Icon(Icons.Outlined.Sync, null, tint = MaterialTheme.colorScheme.primary) },
                modifier = Modifier.clickable(enabled = !vm.syncing) { vm.sync() })
            ListItem(headlineContent = { Text("Sign out") },
                supportingContent = { Text("Tasks remain on this device") },
                leadingContent = { Icon(Icons.AutoMirrored.Outlined.Logout, null) }, modifier = Modifier.clickable { vm.signOut() })
        } else {
            ListItem(headlineContent = { Text(if (vm.syncing) "Connecting…" else "Sign in to sync") },
                supportingContent = { Text("Use your WorkerCat account across devices") },
                leadingContent = { Icon(Icons.Outlined.CloudSync, null, tint = MaterialTheme.colorScheme.primary) },
                modifier = Modifier.clickable(enabled = !vm.syncing) { vm.startLogin() })
        }
        ListItem(headlineContent = { Text("About CatDo") }, supportingContent = { Text("A little more organized. A little more room to breathe.") },
            leadingContent = { Icon(Icons.Outlined.Pets, null) })
    }
}
