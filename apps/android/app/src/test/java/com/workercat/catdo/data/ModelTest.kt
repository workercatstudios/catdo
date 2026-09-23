package com.workercat.catdo.data

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.json.JSONObject
import java.time.LocalDate

class ModelTest {
    private val workspace = Workspace("2f667c37-c27a-4788-b70f-3709ed37e783", "Personal")
    private val task = Task(
        id = "7c65f7e8-9732-446b-b653-8953b9fcc080",
        workspaceId = workspace.id,
        title = "Renew pass",
        scheduled = "2026-01-31",
        due = "2026-02-02",
        recurrence = Recurrence("Months", monthDay = 31),
    )

    @Test fun monthlyRecurrenceRetainsAnchorAndDeadlineOffset() {
        val first = AppData(listOf(workspace), tasks = listOf(task)).complete(task.id, "2026-01-31")
        assertEquals("2026-02-28", first.tasks.single().scheduled)
        assertEquals("2026-03-02", first.tasks.single().due)
        assertEquals(task, first.history.single().task)
        val second = first.complete(task.id, "2026-02-28")
        assertEquals("2026-03-31", second.tasks.single().scheduled)
    }

    @Test fun mergeKeepsIndependentEditsAndFlagsSameRecord() {
        val base = AppData(listOf(workspace), tasks = listOf(task))
        val local = base.copy(tasks = listOf(task.copy(title = "Renew travel pass")))
        val extra = task.copy(id = "2258c159-dfb0-4961-862c-506ef212fd4f", title = "Get stamps")
        val remote = base.copy(tasks = base.tasks + extra)
        val combined = mergeData(base, local, remote)
        assertTrue(combined.conflicts.isEmpty())
        assertEquals(setOf("Renew travel pass", "Get stamps"), combined.data.tasks.map { it.title }.toSet())
        val conflict = mergeData(base, local, base.copy(tasks = listOf(task.copy(title = "Renew card"))))
        assertEquals(listOf("Renew travel pass"), conflict.conflicts)
        assertEquals("Renew card", mergeData(base, local, base.copy(tasks = listOf(task.copy(title = "Renew card"))), true).data.tasks.single().title)
    }

    @Test fun fixedWeekdayRecurrenceSkipsMissedOccurrences() {
        val next = nextDate(Recurrence("Weekdays", weekdays = listOf(0, 2, 4)),
            LocalDate.parse("2026-09-21"), LocalDate.parse("2026-09-24"))
        assertEquals(LocalDate.parse("2026-09-25"), next)
    }

    @Test fun jsonContractRoundTripsNativeFields() {
        val source = AppData(listOf(workspace), tasks = listOf(task))
        val wire = DataJson.encode(source)
        val encodedTask = wire.getJSONArray("tasks").getJSONObject(0)
        assertEquals(task.workspaceId, encodedTask.getString("workspace_id"))
        assertTrue(encodedTask.isNull("project_id"))
        assertEquals("Months", encodedTask.getJSONObject("recurrence").getString("unit"))
        assertEquals(source, DataJson.decode(JSONObject(wire.toString())))
    }
}
