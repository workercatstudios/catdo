package com.workercat.catdo.ui

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.keyframes
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.LifecycleStartEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.clerk.api.Clerk
import com.clerk.ui.auth.AuthView
import com.google.firebase.messaging.FirebaseMessaging
import com.workercat.catdo.Diagnostics
import com.workercat.catdo.data.*
import com.workercat.catdo.sync.TERMS_REVIEW_URL
import com.workercat.catdo.ui.kirakira.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.time.LocalDate
import java.time.format.DateTimeFormatter

private val mainSections = listOf(Section.Today, Section.Inbox, Section.Upcoming, Section.Projects)

/// Each view carries its own Kirakira hue so icons read at a glance.
@Composable
internal fun Section.hue(): Hue = when (this) {
    Section.Today -> Accents.today()
    Section.Inbox -> Accents.inbox()
    Section.Upcoming -> Accents.upcoming()
    Section.Calendar -> Accents.calendar()
    Section.Completed -> Accents.done()
    Section.Projects -> Accents.projects()
    Section.Search -> Accents.search()
    Section.Settings -> Kirakira.colors.let { Hue(it.mutedInk, it.mutedInk, it.mutedInk) }
}

internal fun Section.icon(): ImageVector = when (this) {
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
    // Collected rather than keyed on the prompt: clearing it must not cancel the toast it just showed.
    LaunchedEffect(Unit) {
        snapshotFlow { vm.undoPrompt }.filterNotNull().collectLatest { prompt ->
            vm.undoPrompt = null
            if (snackbar.showSnackbar(prompt, actionLabel = "Undo", duration = SnackbarDuration.Short) == SnackbarResult.ActionPerformed) vm.undo()
        }
    }


    if (vm.authOpen) {
        val ready by vm.clerkReady.collectAsStateWithLifecycle()
        val error by vm.clerkError.collectAsStateWithLifecycle()
        when {
            ready -> AuthView(
                onDismiss = vm::closeAuth,
                onAuthComplete = vm::authComplete,
            )
            else -> Column(Modifier.fillMaxSize().background(Kirakira.colors.paper).safeDrawingPadding().padding(32.dp),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally) {
                if (error == null) {
                    IdleCat(size = 140.dp, mood = IdleCatMood.Happy)
                    Spacer(Modifier.height(16.dp))
                    LinearProgressIndicator(Modifier.width(120.dp).clip(CircleShape), color = MaterialTheme.colorScheme.primary,
                        trackColor = Kirakira.colors.blush)
                    Spacer(Modifier.height(16.dp))
                    Text("Connecting to sign-in…", style = MaterialTheme.typography.titleMedium)
                } else {
                    IdleCat(size = 140.dp, mood = IdleCatMood.Sleepy)
                    Spacer(Modifier.height(16.dp))
                    Text("Can't reach sign-in", style = MaterialTheme.typography.headlineSmall)
                    Spacer(Modifier.height(8.dp))
                    Text("Check your connection or Private DNS, then try again.", color = Kirakira.colors.mutedInk,
                        style = MaterialTheme.typography.bodyMedium)
                    Spacer(Modifier.height(20.dp))
                    PopButton(onClick = { Clerk.reinitialize() }, size = PopSize.Large) { Text("Try again") }
                }
                Spacer(Modifier.height(8.dp))
                PopButton(onClick = vm::closeAuth, variant = PopVariant.Ghost) { Text("Back to CatDo") }
            }
        }
        return
    }

    val kk = Kirakira.colors
    ModalNavigationDrawer(
        drawerState = drawer,
        scrimColor = Color.Black.copy(alpha = 0.4f),
        drawerContent = {
            ModalDrawerSheet(modifier = Modifier.width(310.dp), drawerContainerColor = kk.sidebar,
                drawerShape = RoundedCornerShape(topEnd = 28.dp, bottomEnd = 28.dp)) {
                Column(Modifier.weight(1f).verticalScroll(rememberScrollState())) {
                    Row(Modifier.padding(start = 20.dp, end = 16.dp, top = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                        IdleCat(size = 64.dp, animated = drawer.targetValue == DrawerValue.Open || drawer.currentValue == DrawerValue.Open)
                        Text("CatDo", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.ExtraBold,
                            modifier = Modifier.padding(start = 10.dp).semantics { heading() })
                    }
                    WorkspacePill(workspace.name, Modifier.padding(horizontal = 16.dp, vertical = 8.dp)) { vm.nameDialog = "switch" }
                    Spacer(Modifier.height(4.dp))
                    val todayCount = data.tasks.count { it.workspaceId == workspace.id && it.isActive(data) && it.isToday(today()) }
                    listOf(Section.Today, Section.Inbox, Section.Upcoming, Section.Calendar, Section.Completed, Section.Search).forEach { section ->
                        DrawerRow(section.name, section.icon(), section.hue().text, selected = vm.section == section,
                            badge = if (section == Section.Today && todayCount > 0) todayCount else null) {
                            vm.select(section); scope.launch { drawer.close() }
                        }
                    }
                    HorizontalDivider(Modifier.padding(horizontal = 24.dp, vertical = 12.dp), color = kk.border)
                    Row(Modifier.fillMaxWidth().padding(start = 28.dp, end = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                        Text("Projects", Modifier.weight(1f).semantics { heading() }, style = MaterialTheme.typography.labelSmall,
                            color = kk.mutedInk)
                        PopIconButton(Icons.Default.Add, "New project", { vm.nameDialog = "project"; scope.launch { drawer.close() } },
                            tint = kk.mutedInk, iconSize = 20.dp)
                    }
                    data.projects.filter { it.workspaceId == workspace.id && !it.archived }.forEach { project ->
                        DrawerRow(project.name, null, Accents.project(project.id),
                            selected = vm.section == Section.Projects && vm.projectId == project.id) {
                            vm.select(Section.Projects, project.id); scope.launch { drawer.close() }
                        }
                    }
                    Spacer(Modifier.height(16.dp))
                    DrawerRow("Settings", Icons.Outlined.Settings, kk.mutedInk, selected = vm.section == Section.Settings) {
                        vm.select(Section.Settings); scope.launch { drawer.close() }
                    }
                    Spacer(Modifier.height(20.dp))
                }
            }
        },
    ) {
        Scaffold(
            containerColor = kk.paper,
            topBar = {
                TopAppBar(
                    expandedHeight = 56.dp,
                    title = { WorkspacePill(workspace.name, compact = true) { vm.nameDialog = "switch" } },
                    navigationIcon = { PopIconButton(Icons.Outlined.Menu, "Open menu", { scope.launch { drawer.open() } }) },
                    actions = { PopIconButton(Icons.Outlined.Search, "Search tasks", { vm.select(Section.Search) }) },
                    colors = TopAppBarDefaults.topAppBarColors(containerColor = kk.paper, titleContentColor = kk.ink,
                        navigationIconContentColor = kk.ink, actionIconContentColor = kk.ink),
                )
            },
            bottomBar = {
                if (!wide) PopTabBar(
                    tabs = mainSections.map { PopTab(it.name, it.icon(), it.hue().text) },
                    selected = mainSections.indexOf(vm.section).takeIf { vm.section != Section.Projects || vm.projectId == null } ?: -1,
                    onSelect = { vm.select(mainSections[it]) },
                )
            },
            snackbarHost = { SnackbarHost(snackbar) { PopToast(it) } },
            floatingActionButton = {
                val showAdd = when (vm.section) {
                    Section.Today, Section.Inbox, Section.Upcoming -> true
                    Section.Projects -> vm.projectId != null
                    else -> false
                }
                val reduced = Kirakira.reducedMotion
                AnimatedVisibility(showAdd,
                    enter = if (reduced) fadeIn() else scaleIn(keyframes {
                        durationMillis = 400
                        0.6f at 0 using KkEase.CssOut
                        1.06f at 200 using KkEase.CssInOut
                        0.98f at 300 using KkEase.CssInOut
                    }, initialScale = 0.6f) + fadeIn(tween(120)),
                    exit = if (reduced) fadeOut() else scaleOut(tween(150, easing = KkEase.CssIn), targetScale = 0.9f) + fadeOut(tween(150))) {
                    PopButton(
                        onClick = { vm.newTask(if (vm.section == Section.Today) today() else null) },
                        size = PopSize.Large, icon = Icons.Outlined.Add, elevated = true,
                        modifier = Modifier.height(56.dp),
                    ) { Text("Add task") }
                }
            },
        ) { padding ->
            Row(Modifier.fillMaxSize().padding(padding)) {
                if (wide) NavigationRail(containerColor = kk.sidebar) {
                    mainSections.forEach { section ->
                        NavigationRailItem(
                            selected = vm.section == section && (section != Section.Projects || vm.projectId == null),
                            onClick = { vm.select(section) },
                            icon = { Icon(section.icon(), null) }, label = { Text(section.name) },
                            colors = NavigationRailItemDefaults.colors(
                                selectedIconColor = section.hue().text, selectedTextColor = kk.ink,
                                unselectedIconColor = kk.mutedInk, unselectedTextColor = kk.mutedInk,
                                indicatorColor = kk.blush,
                            ),
                        )
                    }
                }
                Box(Modifier.weight(1f).fillMaxHeight(), contentAlignment = Alignment.TopCenter) {
                val reduced = Kirakira.reducedMotion
                AnimatedContent(
                    targetState = vm.section to vm.projectId,
                    transitionSpec = {
                        // Pop Tabs' panel: in from 0.5rem below on Kirakira's ease-out; the old one goes at once.
                        (fadeIn(tween(250, easing = KkEase.Out)) +
                            (if (reduced) slideInVertically { 0 } else slideInVertically(tween(250, easing = KkEase.Out)) { it / 60 })) togetherWith
                            fadeOut(tween(100))
                    },
                    label = "section",
                    modifier = Modifier.widthIn(max = 840.dp).fillMaxSize(),
                ) { (section, projectId) ->
                Column(Modifier.fillMaxSize()) {
                when (section) {
                    Section.Projects -> if (projectId == null) ProjectOverview(data, workspace, vm) else {
                        val tint = Accents.project(projectId)
                        TaskSection(data, vm, section, data.projects.firstOrNull { it.id == projectId }?.name ?: "Project",
                            data.tasks.filter { it.workspaceId == workspace.id && it.projectId == projectId && it.completedAt == null },
                            hue = Hue(tint, tint, tint))
                    }
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
            "switch" -> PopDialog(
                onDismissRequest = { vm.nameDialog = null }, title = "Workspaces",
                buttons = { dialog ->
                    PopButton(onClick = { dialog.dismiss { vm.nameDialog = "workspace" } }, variant = PopVariant.Ghost, icon = Icons.Default.Add) {
                        Text("New workspace")
                    }
                    PopButton(onClick = { dialog.dismiss { vm.nameDialog = null } }) { Text("Done") }
                },
            ) { dialog ->
                data.workspaces.filter { !it.archived }.forEach { item ->
                    val current = item.id == workspace.id
                    Row(
                        Modifier.fillMaxWidth().heightIn(min = 52.dp)
                            .popClickable({ dialog.dismiss { vm.workspaceId = item.id; vm.select(Section.Today); vm.nameDialog = null } },
                                shape = MaterialTheme.shapes.small, selected = if (current) kk.blush else Color.Transparent)
                            .semantics { selected = current }
                            .padding(horizontal = 14.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(item.name, Modifier.weight(1f), style = MaterialTheme.typography.bodyLarge, color = kk.ink,
                            fontWeight = if (current) FontWeight.Bold else FontWeight.Medium, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        if (current) Icon(Icons.Outlined.Check, null, Modifier.size(20.dp).popIn(from = 0f), tint = MaterialTheme.colorScheme.primary)
                    }
                }
            }
            else -> NameDialog(if (type == "workspace") "New workspace" else "New project",
                onDismiss = { vm.nameDialog = null }, onCreate = {
                    if (type == "workspace") vm.createWorkspace(it) else vm.createProject(it)
                    vm.nameDialog = null
                })
        }
    }

    state.conflict?.let {
        PopDialog(
            onDismissRequest = {}, dismissible = false, title = "Choose which changes to keep",
            art = { IdleCat(size = 96.dp) },
            buttons = {
                // These act at once: the dialog leaves when the conflict is resolved.
                PopButton(onClick = { vm.resolveConflict(true) }, variant = PopVariant.Outline) { Text("Keep cloud changes") }
                PopButton(onClick = { vm.resolveConflict(false) }) { Text("Keep this device") }
            },
        ) {
            Text("Tasks were changed on this device and another device. Your choice applies to the conflicting records. Other changes are combined.",
                style = MaterialTheme.typography.bodyMedium)
        }
    }
}

@Composable
private fun NameDialog(title: String, onDismiss: () -> Unit, onCreate: (String) -> Unit) {
    var name by remember(title) { mutableStateOf("") }
    PopDialog(
        onDismissRequest = onDismiss, title = title,
        buttons = { dialog ->
            PopButton(onClick = { dialog.dismiss(onDismiss) }, variant = PopVariant.Ghost) { Text("Cancel") }
            PopButton(onClick = { dialog.dismiss { onCreate(name) } }, enabled = name.trim().isNotBlank()) { Text("Create") }
        },
    ) { dialog ->
        PopTextField(name, { name = it }, label = "Name", modifier = Modifier.padding(top = 4.dp),
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
            keyboardActions = KeyboardActions(onDone = { if (name.trim().isNotBlank()) dialog.dismiss { onCreate(name) } }))
    }
}

/** The workspace switcher: a quiet pill with the name and a chevron. */
@Composable
private fun WorkspacePill(name: String, modifier: Modifier = Modifier, compact: Boolean = false, onClick: () -> Unit) {
    val kk = Kirakira.colors
    Row(
        modifier
            .popClickable(onClick, shape = CircleShape, selected = if (compact) Color.Transparent else kk.muted, pressed = kk.blush, sink = 0.96f)
            .heightIn(min = 44.dp)
            .padding(start = if (compact) 10.dp else 16.dp, end = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(name, style = if (compact) MaterialTheme.typography.titleMedium else MaterialTheme.typography.bodyMedium,
            fontWeight = FontWeight.Bold, maxLines = 1, overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f, fill = !compact))
        Icon(Icons.Outlined.KeyboardArrowDown, "Switch workspace", Modifier.padding(start = 2.dp).size(20.dp), tint = kk.mutedInk)
    }
}

/** A drawer destination: blush when selected, with a pink pill at its edge and the view's hue on its icon. */
@Composable
private fun DrawerRow(label: String, icon: ImageVector?, tint: Color, selected: Boolean, badge: Int? = null, onClick: () -> Unit) {
    val kk = Kirakira.colors
    val indicator by animateDpAsState(if (selected) 4.dp else 0.dp, tween(250, easing = KkEase.Spring), label = "indicator")
    Row(
        Modifier.padding(horizontal = 12.dp, vertical = 1.dp).fillMaxWidth().height(48.dp)
            .popClickable(onClick, shape = MaterialTheme.shapes.small, selected = if (selected) kk.blush else Color.Transparent, role = Role.Tab)
            .semantics { this.selected = selected }
            .padding(end = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.width(16.dp).fillMaxHeight(), contentAlignment = Alignment.CenterStart) {
            Box(Modifier.padding(start = 4.dp).size(indicator, 18.dp).background(MaterialTheme.colorScheme.primary, CircleShape))
        }
        if (icon != null) Icon(icon, null, Modifier.size(20.dp), tint = tint)
        else Box(Modifier.size(20.dp), contentAlignment = Alignment.Center) { Box(Modifier.size(10.dp).background(tint, CircleShape)) }
        Text(label, Modifier.weight(1f).padding(start = 14.dp), style = MaterialTheme.typography.bodyMedium, color = kk.ink,
            fontWeight = if (selected) FontWeight.Bold else FontWeight.Medium, maxLines = 1, overflow = TextOverflow.Ellipsis)
        if (badge != null) PopBadge(badge.toString())
    }
}

@Composable
internal fun SectionHeading(title: String, count: Int = 0, detail: String? = null, icon: ImageVector? = null,
    hue: Hue = Hue(MaterialTheme.colorScheme.primary, MaterialTheme.colorScheme.primary, MaterialTheme.colorScheme.primary),
    onAdd: (() -> Unit)? = null) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(top = 12.dp, bottom = 16.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            if (icon != null) Box(Modifier.padding(end = 14.dp).size(44.dp).popIn(from = 0.6f)
                .background(hue.fill.copy(alpha = 0.22f), RoundedCornerShape(14.dp)), contentAlignment = Alignment.Center) {
                Icon(icon, null, Modifier.size(24.dp), tint = hue.text)
            }
            Row(Modifier.weight(1f), verticalAlignment = Alignment.CenterVertically) {
                Text(title, style = MaterialTheme.typography.headlineLarge, modifier = Modifier.weight(1f, fill = false).semantics { heading() },
                    maxLines = 1, overflow = TextOverflow.Ellipsis)
                if (count > 0) PopBadge(count.toString(), Modifier.padding(start = 10.dp).popIn(delay = 120, from = 0.4f))
            }
            if (onAdd != null) PopButton(onClick = onAdd, size = PopSize.Small, icon = Icons.Outlined.Add) { Text("Add task") }
        }
        if (detail != null) Text(detail, style = MaterialTheme.typography.bodySmall,
            color = Kirakira.colors.mutedInk,
            modifier = Modifier.padding(top = 2.dp, start = if (icon != null) 58.dp else 0.dp))
    }
}

@Composable
internal fun CountPill(count: Int, modifier: Modifier = Modifier, selected: Boolean = false) =
    PopBadge(count.toString(), modifier, selected = selected)

/** Rows settle in with a short fade and lift, staggered a little; once per row, not on every scroll. */
@Composable
internal fun Modifier.rowEnter(key: String, index: Int, seen: MutableSet<String>): Modifier {
    val reduced = Kirakira.reducedMotion
    val fresh = remember(key) { seen.add(key) }
    if (!fresh || reduced) return this
    val t = remember(key) { androidx.compose.animation.core.Animatable(0f) }
    LaunchedEffect(key) { t.animateTo(1f, tween(260, delayMillis = 30 * index.coerceAtMost(8), easing = KkEase.Out)) }
    return graphicsLayer { alpha = t.value; translationY = (1f - t.value) * 10.dp.toPx() }
}

@Composable
private fun TaskSection(data: AppData, vm: CatDoViewModel, section: Section, title: String, tasks: List<Task>,
    hue: Hue = section.hue()) {
    val sorted = tasks.sortedWith(compareBy<Task> { it.scheduled ?: it.due ?: "9999-12-31" }.thenBy { it.createdAt })
    val isToday = section == Section.Today
    val overdue = if (isToday) sorted.filter { it.due != null && it.due < today() } else emptyList()
    val remaining = sorted.filterNot { it in overdue }
    val seen = remember { mutableSetOf<String>() }
    LazyColumn(contentPadding = PaddingValues(bottom = 112.dp)) {
        item(key = "heading") {
            SectionHeading(title, sorted.size, icon = section.icon(), hue = hue,
                detail = if (isToday) LocalDate.now().format(DateTimeFormatter.ofPattern("EEEE, MMMM d")) else null)
        }
        if (sorted.isEmpty()) item(key = "empty") {
            if (section == Section.Completed) EmptyState("Small steps add up.", "Completed tasks will show up here.",
                mood = IdleCatMood.Idle, modifier = Modifier.animateItem())
            else EmptyState("A little breathing room.", "Add something to do, or enjoy the clear space.",
                mood = IdleCatMood.Sleepy, modifier = Modifier.animateItem())
        }
        if (overdue.isNotEmpty()) {
            item(key = "overdue") { TaskGroupHeading("Overdue", overdue.size, overdue = true, modifier = Modifier.animateItem()) }
            itemsIndexed(overdue, key = { _, task -> task.id }) { index, task ->
                TaskRow(task, data, { vm.edit(task) }, { vm.complete(task.id) }, showScheduled = false,
                    modifier = Modifier.animateItem().rowEnter(task.id, index, seen))
            }
            if (remaining.isNotEmpty()) item(key = "today") { TaskGroupHeading("Today", remaining.size, modifier = Modifier.animateItem()) }
        }
        itemsIndexed(remaining, key = { _, task -> task.id }) { index, task ->
            TaskRow(task, data, { vm.edit(task) }, { vm.complete(task.id) }, showScheduled = !isToday,
                modifier = Modifier.animateItem().rowEnter(task.id, index + overdue.size, seen))
        }
    }
}

@Composable
private fun TaskGroupHeading(title: String, count: Int, overdue: Boolean = false, modifier: Modifier = Modifier) {
    val kk = Kirakira.colors
    val tone = if (overdue) Accents.overdue().text else Accents.today().text
    Column(modifier) {
        Row(Modifier.fillMaxWidth().padding(start = 20.dp, end = 20.dp, top = 12.dp, bottom = 8.dp).semantics(mergeDescendants = true) { heading() },
            verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Icon(if (overdue) Icons.Outlined.ErrorOutline else Icons.Outlined.WbSunny, null, Modifier.size(15.dp), tint = tone)
            Text(title, style = MaterialTheme.typography.labelLarge, color = if (overdue) tone else kk.ink)
            Text(count.toString(), style = MaterialTheme.typography.labelMedium, color = kk.mutedInk)
        }
        HorizontalDivider(Modifier.padding(horizontal = 20.dp), color = kk.border)
    }
}

@Composable
fun TaskRow(task: Task, data: AppData, onOpen: () -> Unit, onComplete: () -> Unit, showScheduled: Boolean = true,
    modifier: Modifier = Modifier) {
    val kk = Kirakira.colors
    val reduced = Kirakira.reducedMotion
    // Let the check pop, draw its tick and burst before the row leaves the list.
    var completing by remember(task.id) { mutableStateOf(false) }
    LaunchedEffect(completing) {
        if (completing) { delay(if (reduced) 200 else 600); onComplete(); completing = false }
    }
    val completed = task.completedAt != null || completing
    val muted = kk.mutedInk
    val overdue = task.due != null && task.due < today() && !completed
    val project = task.projectId?.let { id -> data.projects.firstOrNull { it.id == id } }
    val subtasks = data.tasks.count { it.parentId == task.id }
    val dueToday = task.due != null && task.due == today() && !completed
    val hasMeta = task.due != null || (showScheduled && task.scheduled != null) || project != null || task.recurrence != null || subtasks > 0
    val titleColor by animateColorAsState(if (completed) muted else kk.ink, tween(220), label = "title")
    Column(modifier.fillMaxWidth().padding(horizontal = 8.dp)) {
        Row(Modifier.fillMaxWidth().heightIn(min = 60.dp).popClickable(onOpen, shape = MaterialTheme.shapes.medium, sink = 0.985f)
            .padding(end = 12.dp), verticalAlignment = Alignment.Top) {
            PopCheck(
                checked = completed,
                onCheckedChange = { if (task.completedAt != null) onComplete() else if (!completing) completing = true },
                contentDescription = if (task.completedAt != null) "Restore ${task.title}" else "Complete ${task.title}",
                modifier = Modifier.padding(top = 4.dp),
            )
            Column(Modifier.weight(1f).padding(top = 13.dp, bottom = 12.dp)) {
                Text(task.title, style = MaterialTheme.typography.bodyLarge, color = titleColor,
                    textDecoration = if (completed) TextDecoration.LineThrough else TextDecoration.None)
                if (task.notes.isNotBlank()) Text(task.notes, maxLines = 1, overflow = TextOverflow.Ellipsis,
                    style = MaterialTheme.typography.bodySmall, color = muted, modifier = Modifier.padding(top = 1.dp))
                if (hasMeta) FlowRow(Modifier.padding(top = 6.dp), horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalArrangement = Arrangement.spacedBy(4.dp), itemVerticalAlignment = Alignment.CenterVertically) {
                    task.due?.let { due ->
                        when {
                            overdue -> MetaChip(Icons.Outlined.Flag, "Due ${formatDay(due)}", Accents.overdue())
                            dueToday -> MetaChip(Icons.Outlined.Flag, "Due ${formatDay(due)}", Accents.today())
                            else -> MetaChip(Icons.Outlined.Flag, "Due ${formatDay(due)}", null)
                        }
                    }
                    if (showScheduled) task.scheduled?.let { MetaLabel(Icons.Outlined.Event, formatDay(it), Accents.scheduled().text) }
                    project?.let {
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                            Box(Modifier.size(8.dp).background(Accents.project(it.id), CircleShape))
                            Text(it.name, style = MaterialTheme.typography.bodySmall, fontSize = 12.sp, maxLines = 1, overflow = TextOverflow.Ellipsis,
                                color = muted, modifier = Modifier.widthIn(max = 140.dp))
                        }
                    }
                    task.recurrence?.let { MetaLabel(Icons.Outlined.Repeat, "Repeats", Accents.repeat().text) }
                    if (subtasks > 0) MetaLabel(Icons.Outlined.AccountTree, subtasks.toString(), muted)
                }
            }
        }
    }
}

/** A deadline chip: a soft pill in the meaning's hue when it needs attention, plain otherwise. */
@Composable
private fun MetaChip(icon: ImageVector, label: String, hue: Hue?) {
    if (hue == null) return MetaLabel(icon, label, Kirakira.colors.mutedInk)
    Row(Modifier.background(hue.fill.copy(alpha = 0.18f), CircleShape).padding(horizontal = 8.dp, vertical = 2.dp),
        verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        Icon(icon, null, Modifier.size(12.dp), tint = hue.text)
        Text(label, style = MaterialTheme.typography.labelMedium, maxLines = 1, color = hue.text)
    }
}

@Composable
private fun MetaLabel(icon: ImageVector, label: String, tint: Color) {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        Icon(icon, null, Modifier.size(13.dp), tint = tint)
        Text(label, style = MaterialTheme.typography.bodySmall, fontSize = 12.sp, maxLines = 1, color = tint)
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

/** An empty state: the Idle Cat in a mood that fits, with a line on what to do next. */
@Composable
internal fun EmptyState(title: String, subtitle: String, modifier: Modifier = Modifier, mood: IdleCatMood = IdleCatMood.Sleepy,
    sparkle: Boolean = false) {
    Column(modifier.fillMaxWidth().padding(horizontal = 24.dp, vertical = 32.dp).popIn(), horizontalAlignment = Alignment.CenterHorizontally) {
        IdleCat(Modifier.then(if (sparkle) Modifier.sparkles(count = 6, size = 16.dp) else Modifier), size = 132.dp, mood = mood)
        Text(title, style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(top = 14.dp))
        if (subtitle.isNotBlank()) Text(subtitle, style = MaterialTheme.typography.bodySmall,
            color = Kirakira.colors.mutedInk, modifier = Modifier.padding(top = 4.dp))
    }
}

@Composable
private fun ProjectOverview(data: AppData, workspace: Workspace, vm: CatDoViewModel) {
    val kk = Kirakira.colors
    val projects = data.projects.filter { it.workspaceId == workspace.id && !it.archived }
    val seen = remember { mutableSetOf<String>() }
    LazyColumn(contentPadding = PaddingValues(bottom = 24.dp)) {
        item { SectionHeading("Projects", projects.size, icon = Section.Projects.icon(), hue = Section.Projects.hue()) }
        if (projects.isEmpty()) item { EmptyState("A home for your next idea.", "Add a project with the button below.", mood = IdleCatMood.Idle) }
        itemsIndexed(projects, key = { _, project -> project.id }) { index, project ->
            val count = data.tasks.count { it.projectId == project.id && it.isActive(data) }
            val hue = Accents.project(project.id)
            Row(Modifier.animateItem().rowEnter(project.id, index, seen).fillMaxWidth().padding(horizontal = 8.dp)
                .popClickable({ vm.select(Section.Projects, project.id) }, shape = MaterialTheme.shapes.medium)
                .padding(horizontal = 12.dp).heightIn(min = 60.dp), verticalAlignment = Alignment.CenterVertically) {
                Box(Modifier.size(36.dp).background(hue.copy(alpha = 0.2f), RoundedCornerShape(12.dp)),
                    contentAlignment = Alignment.Center) {
                    Icon(Icons.Outlined.FolderOpen, null, Modifier.size(19.dp), tint = hue)
                }
                Text(project.name, Modifier.weight(1f).padding(horizontal = 14.dp), style = MaterialTheme.typography.bodyLarge,
                    maxLines = 1, overflow = TextOverflow.Ellipsis)
                if (count > 0) PopBadge(count.toString(), selected = false)
                Icon(Icons.Outlined.ChevronRight, null, Modifier.padding(start = 8.dp).size(18.dp), tint = kk.mutedInk)
            }
        }
        item {
            PopButton(onClick = { vm.nameDialog = "project" }, variant = PopVariant.Ghost, icon = Icons.Default.Add,
                modifier = Modifier.padding(start = 12.dp, top = 8.dp)) { Text("New project") }
        }
    }
}

@Composable
private fun SearchScreen(data: AppData, workspace: Workspace, vm: CatDoViewModel) {
    val query = vm.search.trim()
    val tasks = data.tasks.filter { it.workspaceId == workspace.id && it.completedAt == null &&
        query.isNotEmpty() && (it.title.contains(query, true) || it.notes.contains(query, true)) }
    val seen = remember { mutableSetOf<String>() }
    Column {
        SectionHeading("Search", tasks.size, icon = Section.Search.icon(), hue = Section.Search.hue())
        PopTextField(vm.search, { vm.search = it }, label = "Search tasks", icon = Icons.Outlined.Search,
            modifier = Modifier.padding(horizontal = 20.dp), keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search))
        Spacer(Modifier.height(12.dp))
        LazyColumn(contentPadding = PaddingValues(bottom = 100.dp)) {
            itemsIndexed(tasks, key = { _, task -> task.id }) { index, task ->
                TaskRow(task, data, { vm.edit(task) }, { vm.complete(task.id) }, modifier = Modifier.animateItem().rowEnter(task.id, index, seen))
            }
            if (query.isNotEmpty() && tasks.isEmpty()) item { EmptyState("No matching tasks", "Try another word, or switch workspaces.", mood = IdleCatMood.Idle) }
        }
    }
}

@Composable
private fun SettingsScreen(workspace: Workspace, vm: CatDoViewModel, notificationsEnabled: Boolean, notificationBusy: Boolean, onNotificationsChanged: (Boolean) -> Unit) {
    val uriHandler = LocalUriHandler.current
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(bottom = 32.dp)) {
        SectionHeading("Settings", icon = Section.Settings.icon(), hue = Section.Settings.hue())
        SettingsGroup("Workspace", 0) {
            SettingsRow(workspace.name, "", Icons.Outlined.Workspaces, Accents.upcoming(),
                onClick = { vm.nameDialog = "switch" })
            SettingsDivider()
            SettingsRow("New workspace", "", Icons.Outlined.AddCircleOutline, Accents.done(),
                onClick = { vm.nameDialog = "workspace" })
        }
        SettingsGroup("This device", 1) {
            SettingsRow("Available offline", "", Icons.Outlined.OfflinePin, Accents.calendar())
            SettingsDivider()
            SettingsRow("Notifications", "", Icons.Outlined.NotificationsNone, Accents.today(),
                trailing = {
                    PopSwitch(checked = notificationsEnabled, enabled = !notificationBusy, onCheckedChange = onNotificationsChanged,
                        contentDescription = "Notifications")
                })
        }
        SettingsGroup("Account & sync", 2) {
            if (vm.signedIn) {
                if (vm.syncTermsRequired) {
                    SettingsRow("Review terms in browser", "Use the same CatDo account, then tap Sync now", Icons.AutoMirrored.Outlined.OpenInNew,
                        Accents.overdue(),
                        onClick = {
                            runCatching { uriHandler.openUri(TERMS_REVIEW_URL) }
                                .onFailure { vm.message = "Open $TERMS_REVIEW_URL in your browser to review the terms." }
                        })
                    SettingsDivider()
                }
                SettingsRow(if (vm.syncing) "Syncing…" else "Sync now",
                    vm.syncStatus ?: "Keep your WorkerCat devices up to date", Icons.Outlined.Sync, Accents.inbox(),
                    enabled = !vm.syncing, onClick = { vm.sync() })
                SettingsDivider()
                SettingsRow("Sign out", "Tasks remain on this device", Icons.AutoMirrored.Outlined.Logout, null,
                    onClick = { vm.signOut() })
            } else {
                SettingsRow(if (vm.syncing) "Connecting…" else "Sign in to sync",
                    "Use your WorkerCat account across devices", Icons.Outlined.CloudSync, Accents.inbox(),
                    enabled = !vm.syncing && vm.ageConfirmed, onClick = { vm.startLogin() })
                // Asked once per device, ever: a quiet checkbox under the sign-in row.
                if (!vm.ageAnswered) AgeCheck(vm.ageConfirmed, vm::confirmAge)
            }
        }
    }
}

@Composable
private fun AgeCheck(checked: Boolean, onCheckedChange: (Boolean) -> Unit) {
    val interaction = remember { MutableInteractionSource() }
    Row(verticalAlignment = Alignment.CenterVertically,
        modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp)
            .toggleable(value = checked, interactionSource = interaction, indication = null, role = Role.Checkbox,
                onValueChange = onCheckedChange)
            .padding(start = 52.dp, end = 16.dp, bottom = 6.dp)) {
        PopCheck(checked, onCheckedChange = null, contentDescription = null, size = 18.dp, round = false,
            interaction = interaction)
        Text("I'm 13 or older and meet any age rules where I live", style = MaterialTheme.typography.bodySmall,
            color = Kirakira.colors.mutedInk)
    }
}

@Composable
private fun SettingsGroup(title: String, index: Int, content: @Composable ColumnScope.() -> Unit) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 20.dp).popIn(delay = 60 * index, from = 0.92f)) {
        Text(title, style = MaterialTheme.typography.labelLarge, color = Kirakira.colors.mutedInk,
            modifier = Modifier.padding(start = 4.dp, bottom = 8.dp).semantics { heading() })
        Column(Modifier.popCard().clip(RoundedCornerShape(20.dp)), content = content)
    }
}

@Composable
private fun SettingsDivider() = HorizontalDivider(Modifier.padding(start = 64.dp, end = 16.dp), color = Kirakira.colors.border)

@Composable
private fun SettingsRow(title: String, description: String, icon: ImageVector, hue: Hue?, enabled: Boolean = true,
    onClick: (() -> Unit)? = null, trailing: (@Composable () -> Unit)? = null) {
    val kk = Kirakira.colors
    Row(
        Modifier.fillMaxWidth()
            .then(if (onClick != null) Modifier.popClickable(onClick, enabled = enabled, sink = 0.99f) else Modifier)
            .heightIn(min = 60.dp)
            .padding(start = 14.dp, end = if (trailing != null) 4.dp else 14.dp, top = 8.dp, bottom = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(36.dp).background((hue?.fill ?: kk.mutedInk).copy(alpha = if (hue == null) 0.12f else 0.2f), RoundedCornerShape(12.dp)),
            contentAlignment = Alignment.Center) {
            Icon(icon, null, Modifier.size(19.dp), tint = hue?.text ?: kk.mutedInk)
        }
        Column(Modifier.weight(1f).padding(start = 14.dp)) {
            Text(title, style = MaterialTheme.typography.bodyLarge, color = if (enabled) kk.ink else kk.mutedInk)
            if (description.isNotBlank()) Text(description, style = MaterialTheme.typography.bodySmall, color = kk.mutedInk)
        }
        when {
            trailing != null -> trailing()
            onClick != null -> Icon(Icons.Outlined.ChevronRight, null, Modifier.size(18.dp), tint = kk.mutedInk)
        }
    }
}
