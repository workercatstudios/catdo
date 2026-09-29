package com.workercat.catdo.ui

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.selection.toggleable
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import kotlinx.coroutines.delay
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
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.time.LocalDate
import java.time.format.DateTimeFormatter

private val mainSections = listOf(Section.Today, Section.Inbox, Section.Upcoming, Section.Projects)

/// Each view carries its own hue so icons read at a glance.
@Composable
private fun Section.tint(): Color = when (this) {
    Section.Today -> Accents.today()
    Section.Inbox -> Accents.inbox()
    Section.Upcoming -> Accents.upcoming()
    Section.Calendar -> Accents.calendar()
    Section.Completed -> Accents.done()
    else -> MaterialTheme.colorScheme.primary
}

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
        val updates = if (vm.signedIn) scope.launch {
            vm.sync()
            vm.watchRevisions()
        } else null
        onStopOrDispose { updates?.cancel() }
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
                    val todayCount = data.tasks.count { it.workspaceId == workspace.id && it.isActive(data) && it.isToday(today()) }
                    listOf(Section.Today, Section.Inbox, Section.Upcoming, Section.Calendar, Section.Completed, Section.Search).forEach { section ->
                        NavigationDrawerItem(
                            label = { Text(section.name, style = MaterialTheme.typography.bodyMedium) },
                            icon = { Icon(section.icon(), null, Modifier.size(20.dp), tint = section.tint()) },
                            badge = if (section == Section.Today && todayCount > 0) ({ CountPill(todayCount, selected = vm.section == section) }) else null,
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
                            icon = { Icon(Icons.Outlined.FolderOpen, null, Modifier.size(20.dp), tint = Accents.project(project.id)) },
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
                            colors = NavigationBarItemDefaults.colors(
                                selectedIconColor = section.tint(),
                                indicatorColor = section.tint().copy(alpha = 0.14f),
                            ),
                        )
                    }
                }
            },
            snackbarHost = { SnackbarHost(snackbar) },
            floatingActionButton = {
                val showAdd = when (vm.section) {
                    Section.Today, Section.Inbox, Section.Upcoming -> true
                    Section.Projects -> vm.projectId != null
                    else -> false
                }
                AnimatedVisibility(showAdd, enter = scaleIn(spring(Spring.DampingRatioMediumBouncy)) + fadeIn(),
                    exit = scaleOut() + fadeOut()) {
                    ExtendedFloatingActionButton(
                        onClick = { vm.newTask(if (vm.section == Section.Today) today() else null) },
                        icon = { Icon(Icons.Outlined.Add, null) },
                        text = { Text("Add task") },
                        containerColor = MaterialTheme.colorScheme.primary,
                        contentColor = MaterialTheme.colorScheme.onPrimary,
                        shape = MaterialTheme.shapes.large,
                    )
                }
            },
        ) { padding ->
            Row(Modifier.fillMaxSize().padding(padding)) {
                if (wide) NavigationRail(containerColor = MaterialTheme.colorScheme.surfaceContainerLow) {
                    mainSections.forEach { section ->
                        NavigationRailItem(
                            selected = vm.section == section && (section != Section.Projects || vm.projectId == null),
                            onClick = { vm.select(section) },
                            icon = { Icon(section.icon(), null) }, label = { Text(section.name) },
                            colors = NavigationRailItemDefaults.colors(
                                selectedIconColor = section.tint(),
                                indicatorColor = section.tint().copy(alpha = 0.14f),
                            ),
                        )
                    }
                }
                Box(Modifier.weight(1f).fillMaxHeight(), contentAlignment = Alignment.TopCenter) {
                AnimatedContent(
                    targetState = vm.section to vm.projectId,
                    transitionSpec = {
                        (fadeIn(tween(220)) + slideInVertically(tween(260)) { it / 28 }) togetherWith fadeOut(tween(120))
                    },
                    label = "section",
                    modifier = Modifier.widthIn(max = 840.dp).fillMaxSize(),
                ) { (section, projectId) ->
                Column(Modifier.fillMaxSize()) {
                when (section) {
                    Section.Projects -> if (projectId == null) ProjectOverview(data, workspace, vm) else
                        TaskSection(data, vm, section, data.projects.firstOrNull { it.id == projectId }?.name ?: "Project",
                            data.tasks.filter { it.workspaceId == workspace.id && it.projectId == projectId && it.completedAt == null },
                            tint = Accents.project(projectId))
                    Section.Settings -> SettingsScreen(workspace, vm, notificationsEnabled, notificationBusy, toggleNotifications)
                    Section.Search -> SearchScreen(data, workspace, vm)
                    Section.Calendar -> CalendarScreen(data, workspace, vm)
                    else -> {
                        val day = today()
                        val tasks = data.tasks.filter { it.workspaceId == workspace.id && it.isActive(data) }
                        val shown = when (section) {
                            Section.Today -> tasks.filter { it.isToday(day) }
                            Section.Inbox -> tasks.filter { it.projectId == null && it.scheduled == null && it.due == null }
                            Section.Upcoming -> tasks.filter { (it.scheduled ?: it.due)?.let { date -> date > day } == true || (it.due?.let { date -> date > day } == true) }
                            Section.Completed -> data.tasks.filter { it.workspaceId == workspace.id && it.completedAt != null }
                            else -> emptyList()
                        }
                        TaskSection(data, vm, section, section.name, shown)
                    }
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
internal fun SectionHeading(title: String, count: Int = 0, detail: String? = null, icon: ImageVector? = null,
    tint: Color = MaterialTheme.colorScheme.primary, onAdd: (() -> Unit)? = null) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(top = 16.dp, bottom = 16.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            if (icon != null) Box(Modifier.padding(end = 12.dp).size(40.dp)
                .background(tint.copy(alpha = 0.12f), MaterialTheme.shapes.medium), contentAlignment = Alignment.Center) {
                Icon(icon, null, Modifier.size(22.dp), tint = tint)
            }
            Row(Modifier.weight(1f), verticalAlignment = Alignment.CenterVertically) {
                Text(title, style = MaterialTheme.typography.headlineLarge, modifier = Modifier.weight(1f, fill = false),
                    maxLines = 1, overflow = TextOverflow.Ellipsis)
                if (count > 0) CountPill(count, modifier = Modifier.padding(start = 10.dp))
            }
            if (onAdd != null) Button(onClick = onAdd, shape = MaterialTheme.shapes.small,
                contentPadding = PaddingValues(horizontal = 12.dp), modifier = Modifier.height(36.dp)) {
                Icon(Icons.Outlined.Add, null, Modifier.size(16.dp))
                Spacer(Modifier.width(6.dp))
                Text("Add task", style = MaterialTheme.typography.labelMedium)
            }
        }
        if (detail != null) Text(detail, style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(top = 4.dp, start = if (icon != null) 52.dp else 0.dp))
    }
}

@Composable
internal fun CountPill(count: Int, modifier: Modifier = Modifier, selected: Boolean = false) {
    Text(count.toString(), modifier = modifier
        .background(if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surfaceContainerHigh, CircleShape)
        .padding(horizontal = 8.dp, vertical = 2.dp),
        style = MaterialTheme.typography.labelMedium,
        color = if (selected) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurfaceVariant)
}

@Composable
private fun TaskSection(data: AppData, vm: CatDoViewModel, section: Section, title: String, tasks: List<Task>,
    tint: Color = section.tint()) {
    val sorted = tasks.sortedWith(compareBy<Task> { it.scheduled ?: it.due ?: "9999-12-31" }.thenBy { it.createdAt })
    val isToday = section == Section.Today
    val overdue = if (isToday) sorted.filter { it.due != null && it.due < today() } else emptyList()
    val remaining = sorted.filterNot { it in overdue }
    LazyColumn(contentPadding = PaddingValues(bottom = 96.dp)) {
        item(key = "heading") {
            SectionHeading(title, sorted.size, icon = section.icon(), tint = tint,
                detail = if (isToday) LocalDate.now().format(DateTimeFormatter.ofPattern("EEEE, MMMM d")) else null)
        }
        if (sorted.isEmpty()) item(key = "empty") {
            EmptyState(
                if (section == Section.Completed) "Small steps add up." else "A little breathing room.",
                if (section == Section.Completed) "Completed tasks will show up here." else "Add something to do, or enjoy the clear space.",
                modifier = Modifier.animateItem(),
            )
        }
        if (overdue.isNotEmpty()) {
            item(key = "overdue") { TaskGroupHeading("Overdue", overdue.size, overdue = true, modifier = Modifier.animateItem()) }
            items(overdue, key = { it.id }) { task ->
                TaskRow(task, data, { vm.edit(task) }, { vm.complete(task.id) }, showScheduled = false, modifier = Modifier.animateItem())
            }
            if (remaining.isNotEmpty()) item(key = "today") { TaskGroupHeading("Today", remaining.size, modifier = Modifier.animateItem()) }
        }
        items(remaining, key = { it.id }) { task ->
            TaskRow(task, data, { vm.edit(task) }, { vm.complete(task.id) }, showScheduled = !isToday, modifier = Modifier.animateItem())
        }
    }
}

@Composable
private fun TaskGroupHeading(title: String, count: Int, overdue: Boolean = false, modifier: Modifier = Modifier) {
    Column(modifier) {
        Row(Modifier.fillMaxWidth().padding(start = 20.dp, end = 20.dp, top = 12.dp, bottom = 8.dp),
            verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Icon(if (overdue) Icons.Outlined.ErrorOutline else Icons.Outlined.WbSunny, null, Modifier.size(14.dp),
                tint = if (overdue) MaterialTheme.colorScheme.error else Accents.today())
            Text(title, style = MaterialTheme.typography.labelMedium,
                color = if (overdue) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface)
            Text(count.toString(), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        HorizontalDivider(Modifier.padding(horizontal = 20.dp))
    }
}

@Composable
fun TaskRow(task: Task, data: AppData, onOpen: () -> Unit, onComplete: () -> Unit, showScheduled: Boolean = true,
    modifier: Modifier = Modifier) {
    // Let the check fill before the row leaves the list.
    var completing by remember(task.id) { mutableStateOf(false) }
    LaunchedEffect(completing) {
        if (completing) { delay(280); onComplete(); completing = false }
    }
    val completed = task.completedAt != null || completing
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    val overdue = task.due != null && task.due < today() && !completed
    val project = task.projectId?.let { id -> data.projects.firstOrNull { it.id == id } }
    val subtasks = data.tasks.count { it.parentId == task.id }
    val dueToday = task.due != null && task.due == today() && !completed
    val hasMeta = task.due != null || (showScheduled && task.scheduled != null) || project != null || task.recurrence != null || subtasks > 0
    val titleColor by animateColorAsState(if (completed) muted else MaterialTheme.colorScheme.onSurface, tween(220), label = "title")
    Column(modifier.fillMaxWidth().padding(horizontal = 8.dp)) {
        Row(Modifier.fillMaxWidth().heightIn(min = 60.dp).clip(MaterialTheme.shapes.medium).clickable(onClick = onOpen)
            .padding(end = 12.dp), verticalAlignment = Alignment.Top) {
            CompleteCheck(
                completed = completed,
                label = if (task.completedAt != null) "Restore ${task.title}" else "Complete ${task.title}",
                modifier = Modifier.padding(top = 4.dp),
            ) { if (task.completedAt != null) onComplete() else if (!completing) completing = true }
            Column(Modifier.weight(1f).padding(top = 13.dp, bottom = 12.dp)) {
                Text(task.title, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Normal,
                    color = titleColor,
                    textDecoration = if (completed) TextDecoration.LineThrough else TextDecoration.None)
                if (task.notes.isNotBlank()) Text(task.notes, maxLines = 1, overflow = TextOverflow.Ellipsis,
                    style = MaterialTheme.typography.bodySmall, color = muted, modifier = Modifier.padding(top = 2.dp))
                if (hasMeta) Row(Modifier.padding(top = 5.dp), horizontalArrangement = Arrangement.spacedBy(6.dp),
                    verticalAlignment = Alignment.CenterVertically) {
                    task.due?.let { due ->
                        MetaChip(Icons.Outlined.Flag, "Due ${formatDay(due)}",
                            if (overdue) MaterialTheme.colorScheme.error else if (dueToday) Accents.today() else muted,
                            filled = overdue || dueToday)
                    }
                    if (showScheduled) task.scheduled?.let { MetaChip(Icons.Outlined.Event, formatDay(it), Accents.scheduled()) }
                    project?.let {
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                            Box(Modifier.size(7.dp).background(Accents.project(it.id), CircleShape))
                            Text(it.name, fontSize = 11.sp, lineHeight = 16.sp, maxLines = 1, overflow = TextOverflow.Ellipsis,
                                color = muted, modifier = Modifier.widthIn(max = 120.dp))
                        }
                    }
                    task.recurrence?.let { MetaChip(Icons.Outlined.Repeat, "Repeats", Accents.repeat()) }
                    if (subtasks > 0) MetaChip(Icons.Outlined.AccountTree, subtasks.toString(), muted)
                }
            }
        }
    }
}

@Composable
private fun MetaChip(icon: ImageVector, label: String, tint: Color, filled: Boolean = false) {
    Row(Modifier.then(if (filled) Modifier.background(tint.copy(alpha = 0.12f), CircleShape).padding(horizontal = 7.dp, vertical = 2.dp) else Modifier),
        verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        Icon(icon, null, Modifier.size(12.dp), tint = tint)
        Text(label, fontSize = 11.sp, lineHeight = 16.sp, maxLines = 1, color = tint,
            fontWeight = if (filled) FontWeight.Medium else FontWeight.Normal)
    }
}

/// A round check that fills green and springs under the finger.
@Composable
private fun CompleteCheck(completed: Boolean, label: String, modifier: Modifier = Modifier, onClick: () -> Unit) {
    val interaction = remember { MutableInteractionSource() }
    val pressed by interaction.collectIsPressedAsState()
    val scale by animateFloatAsState(
        if (pressed) 0.85f else if (completed) 1.08f else 1f,
        spring(dampingRatio = Spring.DampingRatioMediumBouncy, stiffness = Spring.StiffnessMediumLow), label = "checkScale")
    val done = Accents.done()
    val fill by animateColorAsState(if (completed) done else Color.Transparent, tween(180), label = "checkFill")
    val ring by animateColorAsState(
        if (completed) done else MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.55f),
        tween(180), label = "checkRing")
    Box(modifier.size(48.dp)
        .clickable(interactionSource = interaction, indication = ripple(bounded = false, radius = 24.dp),
            role = Role.Checkbox, onClick = onClick)
        .semantics { contentDescription = label }, contentAlignment = Alignment.Center) {
        Box(Modifier.size(22.dp).scale(scale).background(fill, CircleShape).border(1.5.dp, ring, CircleShape),
            contentAlignment = Alignment.Center) {
            AnimatedVisibility(completed, enter = scaleIn(spring(Spring.DampingRatioMediumBouncy)) + fadeIn(), exit = scaleOut() + fadeOut()) {
                Icon(Icons.Outlined.Check, null, Modifier.size(14.dp), tint = Color.White)
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
private fun EmptyState(title: String, subtitle: String, modifier: Modifier = Modifier) {
    var shown by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { shown = true }
    AnimatedVisibility(shown, enter = fadeIn(tween(360)) + slideInVertically(tween(360)) { it / 6 }, modifier = modifier) {
        Column(Modifier.fillMaxWidth().padding(horizontal = 24.dp, vertical = 40.dp), horizontalAlignment = Alignment.CenterHorizontally) {
            Box(Modifier.size(56.dp).background(MaterialTheme.colorScheme.primary.copy(alpha = 0.1f), CircleShape),
                contentAlignment = Alignment.Center) {
                Icon(Icons.Outlined.Pets, null, Modifier.size(26.dp), tint = MaterialTheme.colorScheme.primary)
            }
            Text(title, style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(top = 16.dp))
            if (subtitle.isNotBlank()) Text(subtitle, style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(top = 4.dp))
        }
    }
}

@Composable
private fun ProjectOverview(data: AppData, workspace: Workspace, vm: CatDoViewModel) {
    val projects = data.projects.filter { it.workspaceId == workspace.id && !it.archived }
    LazyColumn(contentPadding = PaddingValues(bottom = 24.dp)) {
        item { SectionHeading("Projects", projects.size, icon = Section.Projects.icon()) }
        if (projects.isEmpty()) item { EmptyState("A home for your next idea.", "Add a project with the button below.") }
        items(projects, key = { it.id }) { project ->
            val count = data.tasks.count { it.projectId == project.id && it.isActive(data) }
            Row(Modifier.animateItem().fillMaxWidth().padding(horizontal = 8.dp).clip(MaterialTheme.shapes.medium)
                .clickable { vm.select(Section.Projects, project.id) }
                .padding(horizontal = 12.dp).heightIn(min = 56.dp), verticalAlignment = Alignment.CenterVertically) {
                val hue = Accents.project(project.id)
                Box(Modifier.size(32.dp).background(hue.copy(alpha = 0.14f), MaterialTheme.shapes.small),
                    contentAlignment = Alignment.Center) {
                    Icon(Icons.Outlined.FolderOpen, null, Modifier.size(18.dp), tint = hue)
                }
                Text(project.name, Modifier.weight(1f).padding(horizontal = 12.dp), style = MaterialTheme.typography.bodyLarge,
                    maxLines = 1, overflow = TextOverflow.Ellipsis)
                if (count > 0) CountPill(count)
                Icon(Icons.Outlined.ChevronRight, null, Modifier.padding(start = 8.dp).size(18.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
            }
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
        SectionHeading("Search", tasks.size, icon = Section.Search.icon())
        OutlinedTextField(vm.search, { vm.search = it }, modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp),
            placeholder = { Text("Search tasks") }, leadingIcon = { Icon(Icons.Outlined.Search, null) }, singleLine = true, shape = MaterialTheme.shapes.small)
        Spacer(Modifier.height(16.dp))
        LazyColumn(contentPadding = PaddingValues(bottom = 100.dp)) {
            items(tasks, key = { it.id }) { task -> TaskRow(task, data, { vm.edit(task) }, { vm.complete(task.id) }, modifier = Modifier.animateItem()) }
            if (query.isNotEmpty() && tasks.isEmpty()) item { EmptyState("No matching tasks", "Try another word, or switch workspaces.") }
        }
    }
}

@Composable
private fun SettingsScreen(workspace: Workspace, vm: CatDoViewModel, notificationsEnabled: Boolean, notificationBusy: Boolean, onNotificationsChanged: (Boolean) -> Unit) {
    val uriHandler = LocalUriHandler.current
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(bottom = 32.dp)) {
        SectionHeading("Settings", icon = Section.Settings.icon())
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
