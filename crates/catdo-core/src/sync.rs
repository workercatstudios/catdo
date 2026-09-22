//! Account-bound, durable synchronization. Every in-flight request is frozen on
//! disk before transport; retries reuse its ID. Dates/history travel together.
use crate::{Data, Store};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub revision: u64,
    pub data: Data,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pending {
    pub id: Uuid,
    pub base: Data,
    pub data: Data,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncState {
    pub owner: String,
    pub base: Snapshot,
    pub pending: Option<Pending>,
    pub conflict: Option<Snapshot>,
}
#[derive(Clone, Copy)]
pub enum Choice {
    Local,
    Remote,
}

pub fn merge(
    base: &Data,
    local: &Data,
    remote: &Data,
    choice: Option<Choice>,
) -> Result<(Data, Vec<String>)> {
    let base_data = base;
    let local_data = local;
    let remote_data = remote;
    let mut conflicting_tasks = BTreeSet::new();
    let base = serde_json::to_value(base)?;
    let local = serde_json::to_value(local)?;
    let remote = serde_json::to_value(remote)?;
    let mut data = serde_json::Map::new();
    let mut conflicts = Vec::new();
    for collection in ["workspaces", "projects", "tasks", "history"] {
        let records = |value: &Value| -> BTreeMap<String, Value> {
            value[collection]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| (v["id"].as_str().unwrap().to_owned(), v.clone()))
                .collect()
        };
        let b = records(&base);
        let l = records(&local);
        let r = records(&remote);
        let ids: BTreeSet<_> = b.keys().chain(l.keys()).chain(r.keys()).collect();
        let mut result = Vec::new();
        // Preserve remote order, then append local additions.
        let order: Vec<_> = remote[collection]
            .as_array()
            .unwrap()
            .iter()
            .chain(local[collection].as_array().unwrap())
            .map(|v| v["id"].as_str().unwrap().to_owned())
            .collect();
        let mut merged = BTreeMap::new();
        for id in ids {
            let before = b.get(id);
            let mine = l.get(id);
            let theirs = r.get(id);
            let value = if mine == before {
                theirs
            } else if theirs == before || mine == theirs {
                mine
            } else {
                let name = mine
                    .or(theirs)
                    .and_then(|v| v.get("title").or(v.get("name")))
                    .and_then(Value::as_str)
                    .unwrap_or("Completed occurrence");
                conflicts.push(name.to_owned());
                if collection == "tasks" {
                    conflicting_tasks.insert(id.clone());
                }
                match choice {
                    Some(Choice::Remote) => theirs,
                    _ => mine,
                }
            };
            if let Some(value) = value {
                merged.insert(id.clone(), value.clone());
            }
        }
        for id in order {
            if let Some(value) = merged.remove(&id) {
                result.push(value);
            }
        }
        data.insert(collection.into(), Value::Array(result));
    }
    let mut result: Data = serde_json::from_value(Value::Object(data))?;
    let original: BTreeSet<_> = base_data.history.iter().map(|h| h.id).collect();
    for mine in local_data
        .history
        .iter()
        .filter(|h| !original.contains(&h.id))
    {
        if remote_data.history.iter().any(|theirs| {
            !original.contains(&theirs.id) && theirs.task.id == mine.task.id && theirs.id != mine.id
        }) && conflicting_tasks.insert(mine.task.id.to_string())
        {
            conflicts.push(mine.task.title.clone());
        }
    }
    let chosen = match choice {
        Some(Choice::Remote) => remote_data,
        _ => local_data,
    };
    let keep: BTreeSet<_> = chosen.history.iter().map(|h| h.id).collect();
    result.history.retain(|h| {
        !conflicting_tasks.contains(&h.task.id.to_string())
            || original.contains(&h.id)
            || keep.contains(&h.id)
    });
    Ok((result, conflicts))
}

impl Store {
    pub fn sync_state(&self) -> Result<Option<SyncState>> {
        self.preference("sync")
    }
    pub fn bind_account(&self, owner: &str) -> Result<()> {
        self.with_current(|| {
        if let Some(state) = self.sync_state()? {
            ensure!(
                state.owner == owner,
                "This database belongs to another account. Sign in to the original account, or use a separate --data-dir."
            );
        } else {
            self.set_preference(
                "sync",
                &SyncState {
                    owner: owner.into(),
                    base: Snapshot::default(),
                    pending: None,
                    conflict: None,
                },
            )?;
        }
        Ok(())
        })
    }
    pub fn prepare_sync(&self, data: &Data) -> Result<Option<Pending>> {
        self.with_current(|| {
            let mut state = self
                .sync_state()?
                .ok_or_else(|| anyhow::anyhow!("Sign in first."))?;
            ensure!(state.conflict.is_none(), "Resolve the sync conflict first.");
            if state.pending.is_none() && state.base.data.workspaces.is_empty() && pristine(data) {
                return Ok(None);
            }
            if state.pending.is_none() && state.base.data != *data {
                state.pending = Some(Pending {
                    id: Uuid::new_v4(),
                    base: state.base.data.clone(),
                    data: data.clone(),
                });
                self.set_preference("sync", &state)?;
            }
            Ok(state.pending)
        })
    }
    pub fn accept_sync(
        &mut self,
        local: &Data,
        remote: Snapshot,
        acknowledged: bool,
    ) -> Result<Option<Data>> {
        let mut state = self
            .sync_state()?
            .ok_or_else(|| anyhow::anyhow!("Sign in first."))?;
        if acknowledged && let Some(pending) = state.pending.take() {
            state.base.data = pending.data;
        }
        let (merged, conflicts) = if state.base.data.workspaces.is_empty()
            && pristine(local)
            && !remote.data.workspaces.is_empty()
        {
            (remote.data.clone(), Vec::new())
        } else {
            merge(&state.base.data, local, &remote.data, None)?
        };
        if !conflicts.is_empty() || merged.validate().is_err() {
            state.pending = None;
            state.conflict = Some(remote);
            self.with_current(|| self.set_preference("sync", &state))?;
            return Ok(None);
        }
        state.base = remote;
        if state.base.data.workspaces.is_empty() && pristine(&merged) {
            state.pending = Some(Pending {
                id: Uuid::new_v4(),
                base: state.base.data.clone(),
                data: merged.clone(),
            });
        } else {
            state.pending = None;
        }
        state.conflict = None;
        self.save_with_sync(&merged, Some(&state))?;
        Ok(Some(merged))
    }
    pub fn resolve_sync(&mut self, local: &Data, choice: Choice) -> Result<Data> {
        let mut state = self
            .sync_state()?
            .ok_or_else(|| anyhow::anyhow!("Sign in first."))?;
        let remote = state
            .conflict
            .take()
            .ok_or_else(|| anyhow::anyhow!("No conflict to resolve."))?;
        let (mut merged, _) = merge(&state.base.data, local, &remote.data, Some(choice))?;
        if merge(&state.base.data, local, &remote.data, Some(Choice::Local))?
            .0
            .validate()
            .is_err()
            || merge(&state.base.data, local, &remote.data, Some(Choice::Remote))?
                .0
                .validate()
                .is_err()
        {
            merged = match choice {
                Choice::Local => local.clone(),
                Choice::Remote => remote.data.clone(),
            };
        }
        merged.validate()?;
        state.base = remote;
        state.pending = None;
        self.save_with_sync(&merged, Some(&state))?;
        Ok(merged)
    }
}

fn pristine(data: &Data) -> bool {
    data.workspaces.len() == 1
        && data.workspaces[0].name == "Personal"
        && !data.workspaces[0].archived
        && data.projects.is_empty()
        && data.tasks.is_empty()
        && data.history.is_empty()
}
