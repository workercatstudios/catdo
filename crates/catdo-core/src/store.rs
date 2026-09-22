use std::path::Path;

use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, params};
use serde::{Serialize, de::DeserializeOwned};

use crate::Data;

/// A local SQLite store. Domain records are versioned JSON so UI and database
/// migrations are independent. Revision checks prevent two app instances from
/// silently replacing each other's changes.
pub struct Store {
    connection: Connection,
    revision: i64,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path).context("Could not open CatDo's local database")?;
        connection.busy_timeout(std::time::Duration::from_secs(3))?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        let schema: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        ensure!(
            schema <= 1,
            "This database was created by a newer version of CatDo."
        );
        connection.execute_batch(
            "BEGIN;
             CREATE TABLE IF NOT EXISTS metadata (id INTEGER PRIMARY KEY CHECK(id = 1), revision INTEGER NOT NULL);
             INSERT OR IGNORE INTO metadata VALUES (1, 0);
             CREATE TABLE IF NOT EXISTS records (kind TEXT NOT NULL, id TEXT NOT NULL, position INTEGER NOT NULL, body TEXT NOT NULL, PRIMARY KEY(kind, id));
             CREATE TABLE IF NOT EXISTS preferences (key TEXT PRIMARY KEY, body TEXT NOT NULL);
             PRAGMA user_version = 1;
             COMMIT;"
        )?;
        let revision =
            connection.query_row("SELECT revision FROM metadata WHERE id = 1", [], |row| {
                row.get(0)
            })?;
        Ok(Self {
            connection,
            revision,
        })
    }

    pub fn load(&mut self) -> Result<Data> {
        let transaction = self.connection.transaction()?;
        let revision =
            transaction.query_row("SELECT revision FROM metadata WHERE id = 1", [], |r| {
                r.get(0)
            })?;
        let data = Data {
            workspaces: read_records(&transaction, "workspace")?,
            projects: read_records(&transaction, "project")?,
            tasks: read_records(&transaction, "task")?,
            history: read_records(&transaction, "occurrence")?,
        };
        transaction.commit()?;
        self.revision = revision;
        if data.workspaces.is_empty() {
            ensure!(
                data.projects.is_empty() && data.tasks.is_empty() && data.history.is_empty(),
                "The database is missing its workspaces."
            );
            let initial = Data::initial();
            self.save(&initial)?;
            return Ok(initial);
        }
        data.validate().context("The saved task data is invalid")?;
        Ok(data)
    }

    pub fn save(&mut self, data: &Data) -> Result<()> {
        self.save_with_sync(data, None)
    }

    pub(crate) fn save_with_sync(
        &mut self,
        data: &Data,
        sync: Option<&crate::sync::SyncState>,
    ) -> Result<()> {
        data.validate()?;
        let tx = self.connection.transaction()?;
        let changed = tx.execute(
            "UPDATE metadata SET revision = revision + 1 WHERE id = 1 AND revision = ?1",
            [self.revision],
        )?;
        ensure!(
            changed == 1,
            "Another CatDo window changed this database. Restart this window before saving; your changes were not written."
        );
        write_records(&tx, "workspace", &data.workspaces, |v| v.id)?;
        write_records(&tx, "project", &data.projects, |v| v.id)?;
        write_records(&tx, "task", &data.tasks, |v| v.id)?;
        write_records(&tx, "occurrence", &data.history, |v| v.id)?;
        if let Some(sync) = sync {
            tx.execute("INSERT INTO preferences(key,body) VALUES ('sync',?1) ON CONFLICT(key) DO UPDATE SET body=excluded.body", [serde_json::to_string(sync)?])?;
        }
        tx.commit()?;
        self.revision += 1;
        Ok(())
    }

    /// Freeze queue/account metadata only while this window still owns the
    /// revision it loaded. An older window must never upload its stale snapshot.
    pub(crate) fn with_current<T>(&self, operation: impl FnOnce() -> Result<T>) -> Result<T> {
        let tx = self.connection.unchecked_transaction()?;
        let revision: i64 =
            tx.query_row("SELECT revision FROM metadata WHERE id=1", [], |r| r.get(0))?;
        ensure!(
            revision == self.revision,
            "Another CatDo window changed this database. Restart this window before syncing."
        );
        let value = operation()?;
        tx.commit()?;
        Ok(value)
    }

    pub fn preference<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        use rusqlite::OptionalExtension;
        let body: Option<String> = self
            .connection
            .query_row("SELECT body FROM preferences WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()?;
        body.map(|body| Ok(serde_json::from_str(&body)?))
            .transpose()
    }

    pub fn set_preference(&self, key: &str, value: &impl Serialize) -> Result<()> {
        self.connection.execute("INSERT INTO preferences (key, body) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET body = excluded.body", params![key, serde_json::to_string(value)?])?;
        Ok(())
    }
}

fn read_records<T: DeserializeOwned>(db: &Connection, kind: &str) -> Result<Vec<T>> {
    let mut query = db.prepare("SELECT body FROM records WHERE kind = ?1 ORDER BY position")?;
    query
        .query_map([kind], |r| r.get::<_, String>(0))?
        .map(|row| Ok(serde_json::from_str(&row?)?))
        .collect()
}

fn write_records<T: Serialize>(
    db: &Connection,
    kind: &str,
    values: &[T],
    id: impl Fn(&T) -> uuid::Uuid,
) -> Result<()> {
    let ids = values.iter().map(|v| id(v).to_string()).collect::<Vec<_>>();
    db.execute(
        "DELETE FROM records WHERE kind = ?1 AND id NOT IN (SELECT value FROM json_each(?2))",
        params![kind, serde_json::to_string(&ids)?],
    )?;
    let mut statement = db.prepare_cached("INSERT INTO records (kind, id, position, body) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(kind, id) DO UPDATE SET position = excluded.position, body = excluded.body WHERE records.body != excluded.body OR records.position != excluded.position")?;
    for (position, value) in values.iter().enumerate() {
        statement.execute(params![
            kind,
            id(value).to_string(),
            position,
            serde_json::to_string(value)?
        ])?;
    }
    Ok(())
}
