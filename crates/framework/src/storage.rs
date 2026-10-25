use planner_contracts::*;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};
fn db(e: impl std::fmt::Display) -> AppError {
    AppError::new("storage", e.to_string())
}
pub struct Store {
    conn: Mutex<Connection>,
    pub path: PathBuf,
}
pub struct UnitOfWork<'a> {
    conn: &'a Connection,
}
pub struct Repository<'a> {
    conn: &'a Connection,
    owner: String,
}
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct Event {
    pub id: i64,
    pub topic: String,
    pub payload: serde_json::Value,
    pub attempts: u32,
}
impl Store {
    pub fn open(path: &Path, modules: &[ModuleManifest]) -> Result<Self> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).map_err(db)?
        }
        let conn = Connection::open(path).map_err(db)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(db)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS framework_meta(key TEXT PRIMARY KEY,value INTEGER NOT NULL); INSERT OR IGNORE INTO framework_meta VALUES('revision',0); CREATE TABLE IF NOT EXISTS framework_migrations(owner TEXT PRIMARY KEY,version INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS framework_outbox(id INTEGER PRIMARY KEY AUTOINCREMENT,topic TEXT NOT NULL,payload TEXT NOT NULL,attempts INTEGER NOT NULL DEFAULT 0,handled INTEGER NOT NULL DEFAULT 0); CREATE TABLE IF NOT EXISTS framework_commands(id TEXT PRIMARY KEY,result TEXT NOT NULL); CREATE TABLE IF NOT EXISTS framework_audit(id INTEGER PRIMARY KEY AUTOINCREMENT,command TEXT NOT NULL,revision INTEGER NOT NULL,created_at TEXT DEFAULT CURRENT_TIMESTAMP);").map_err(db)?;
        let needs = modules.iter().any(|m| {
            conn.query_row(
                "SELECT version FROM framework_migrations WHERE owner=?",
                [&m.id],
                |r| r.get::<_, u32>(0),
            )
            .optional()
            .ok()
            .flatten()
            .unwrap_or(0)
                < m.migration_version
        });
        if needs && path.exists() {
            conn.backup(
                rusqlite::DatabaseName::Main,
                path.with_extension("pre-migration.sqlite"),
                None,
            )
            .map_err(db)?;
        }
        conn.execute_batch("BEGIN IMMEDIATE").map_err(db)?;
        let migrated = (|| {
            for m in modules {
                valid_owner(&m.id)?;
                let current: u32 = conn
                    .query_row(
                        "SELECT version FROM framework_migrations WHERE owner=?",
                        [&m.id],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(db)?
                    .unwrap_or(0);
                if current > m.migration_version {
                    return Err(AppError::new(
                        "migration_downgrade",
                        "数据版本较新，请使用匹配版本或恢复备份",
                    ));
                }
                if m.migration_version > 1 {
                    return Err(AppError::new("migration_missing", "未注册所需迁移"));
                }
                conn.execute_batch(&format!("CREATE TABLE IF NOT EXISTS {}_records(id TEXT PRIMARY KEY, payload TEXT NOT NULL);",m.id)).map_err(db)?;
                conn.execute("INSERT INTO framework_migrations VALUES(?1,?2) ON CONFLICT(owner) DO UPDATE SET version=excluded.version",params![m.id,m.migration_version]).map_err(db)?;
            }
            Ok(())
        })();
        match migrated {
            Ok(()) => conn.execute_batch("COMMIT").map_err(db)?,
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                return Err(e);
            }
        }
        Ok(Self {
            conn: Mutex::new(conn),
            path: path.to_owned(),
        })
    }
    pub fn read<T>(&self, f: impl FnOnce(&UnitOfWork) -> Result<T>) -> Result<T> {
        let conn = self.conn.lock().map_err(db)?;
        f(&UnitOfWork { conn: &conn })
    }
    pub fn transact<T: Serialize + DeserializeOwned>(
        &self,
        expected: Option<u64>,
        command: &str,
        f: impl FnOnce(&UnitOfWork) -> Result<T>,
    ) -> Result<T> {
        let conn = self.conn.lock().map_err(db)?;
        if let Some(cached) = conn
            .query_row(
                "SELECT result FROM framework_commands WHERE id=?",
                [command],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(db)?
        {
            return serde_json::from_str(&cached).map_err(db);
        }
        conn.execute_batch("BEGIN IMMEDIATE").map_err(db)?;
        let result = (|| {
            let u = UnitOfWork { conn: &conn };
            if let Some(v) = expected {
                if u.revision()? != v {
                    return Err(AppError::new(
                        "stale_version",
                        "数据已变化，请刷新后重新规划",
                    ));
                }
            }
            let result = f(&u)?;
            conn.execute(
                "UPDATE framework_meta SET value=value+1 WHERE key='revision'",
                [],
            )
            .map_err(db)?;
            conn.execute(
                "INSERT INTO framework_commands VALUES(?1,?2)",
                params![command, serde_json::to_string(&result).map_err(db)?],
            )
            .map_err(db)?;
            conn.execute(
                "INSERT INTO framework_audit(command,revision) VALUES(?1,?2)",
                params![command, u.revision()?],
            )
            .map_err(db)?;
            Ok(result)
        })();
        match result {
            Ok(v) => {
                conn.execute_batch("COMMIT").map_err(db)?;
                Ok(v)
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }
    pub fn backup(&self, path: &Path) -> Result<()> {
        if path.canonicalize().ok() == self.path.canonicalize().ok() {
            return Err(AppError::new("backup_path", "备份不能覆盖正在使用的数据库"));
        }
        let c = self.conn.lock().map_err(db)?;
        c.backup(rusqlite::DatabaseName::Main, path, None)
            .map_err(db)
    }
    pub fn restore(&self, path: &Path, modules: &[ModuleManifest]) -> Result<()> {
        if path.canonicalize().ok() == self.path.canonicalize().ok() {
            return Err(AppError::new("backup_path", "不能从正在使用的数据库恢复"));
        }
        let source = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(db)?;
        let integrity: String = source
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .map_err(db)?;
        if integrity != "ok" {
            return Err(AppError::new("invalid_backup", integrity));
        }
        for m in modules {
            let v: u32 = source
                .query_row(
                    "SELECT version FROM framework_migrations WHERE owner=?",
                    [&m.id],
                    |r| r.get(0),
                )
                .map_err(db)?;
            if v != m.migration_version {
                return Err(AppError::new("backup_version", "备份模块版本不匹配"));
            }
        }
        let mut c = self.conn.lock().map_err(db)?;
        let previous_revision: u64 = c
            .query_row(
                "SELECT value FROM framework_meta WHERE key='revision'",
                [],
                |r| r.get(0),
            )
            .map_err(db)?;
        c.backup(
            rusqlite::DatabaseName::Main,
            self.path.with_extension("before-restore.sqlite"),
            None,
        )
        .map_err(db)?;
        c.restore(
            rusqlite::DatabaseName::Main,
            path,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .map_err(db)?;
        c.execute(
            "UPDATE framework_meta SET value=MAX(value+1,?1) WHERE key='revision'",
            [previous_revision + 1],
        )
        .map_err(db)?;
        Ok(())
    }
    pub fn pending_events(&self) -> Result<Vec<Event>> {
        self.read(|u|{let mut q=u.conn.prepare("SELECT id,topic,payload,attempts FROM framework_outbox WHERE handled=0 AND attempts<3 ORDER BY id").map_err(db)?;let rows=q.query_map([],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,u32>(3)?))).map_err(db)?;rows.map(|r|{let(id,topic,payload,attempts)=r.map_err(db)?;Ok(Event{id,topic,payload:serde_json::from_str(&payload).map_err(db)?,attempts})}).collect()})
    }
    pub fn exhausted_events(&self) -> Result<u32> {
        self.read(|u| {
            u.conn
                .query_row(
                    "SELECT COUNT(*) FROM framework_outbox WHERE handled=0 AND attempts>=3",
                    [],
                    |r| r.get(0),
                )
                .map_err(db)
        })
    }
    pub fn event_failed(&self, id: i64) -> Result<()> {
        self.read(|u| {
            u.conn
                .execute(
                    "UPDATE framework_outbox SET attempts=attempts+1 WHERE id=?",
                    [id],
                )
                .map_err(db)?;
            Ok(())
        })
    }
}
impl UnitOfWork<'_> {
    pub fn revision(&self) -> Result<u64> {
        self.conn
            .query_row(
                "SELECT value FROM framework_meta WHERE key='revision'",
                [],
                |r| r.get(0),
            )
            .map_err(db)
    }
    pub fn repo(&self, owner: &str) -> Result<Repository<'_>> {
        valid_owner(owner)?;
        let found: bool = self
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM framework_migrations WHERE owner=?)",
                [owner],
                |r| r.get(0),
            )
            .map_err(db)?;
        if !found {
            return Err(AppError::new("unregistered_module", owner));
        }
        Ok(Repository {
            conn: self.conn,
            owner: owner.into(),
        })
    }
    pub fn emit(&self, topic: &str, payload: &impl Serialize) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO framework_outbox(topic,payload) VALUES(?1,?2)",
                params![topic, serde_json::to_string(payload).map_err(db)?],
            )
            .map_err(db)?;
        Ok(())
    }
    pub fn retry_events(&self) -> Result<()> {
        self.conn
            .execute("UPDATE framework_outbox SET attempts=0 WHERE handled=0", [])
            .map_err(db)?;
        Ok(())
    }
    pub fn acknowledge(&self, ids: &[i64]) -> Result<()> {
        for id in ids {
            self.conn
                .execute("UPDATE framework_outbox SET handled=1 WHERE id=?", [id])
                .map_err(db)?;
        }
        Ok(())
    }
}
fn valid_owner(s: &str) -> Result<()> {
    if s.is_empty() || !s.bytes().all(|c| c.is_ascii_lowercase() || c == b'_') {
        return Err(AppError::new("invalid_module", s));
    }
    Ok(())
}
impl Repository<'_> {
    pub fn get<T: DeserializeOwned>(&self, id: &str) -> Result<Option<T>> {
        let s = self
            .conn
            .query_row(
                &format!("SELECT payload FROM {}_records WHERE id=?", self.owner),
                [id],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(db)?;
        s.map(|s| serde_json::from_str(&s).map_err(db)).transpose()
    }
    pub fn all<T: DeserializeOwned>(&self) -> Result<Vec<T>> {
        let mut q = self
            .conn
            .prepare(&format!(
                "SELECT payload FROM {}_records ORDER BY id",
                self.owner
            ))
            .map_err(db)?;
        let rows = q.query_map([], |r| r.get::<_, String>(0)).map_err(db)?;
        rows.map(|s| serde_json::from_str(&s.map_err(db)?).map_err(db))
            .collect()
    }
    pub fn put(&self, id: &str, value: &impl Serialize) -> Result<()> {
        self.conn.execute(&format!("INSERT INTO {}_records(id,payload) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",self.owner),params![id,serde_json::to_string(value).map_err(db)?]).map_err(db)?;
        Ok(())
    }
    pub fn delete(&self, id: &str) -> Result<()> {
        self.conn
            .execute(
                &format!("DELETE FROM {}_records WHERE id=?", self.owner),
                [id],
            )
            .map_err(db)?;
        Ok(())
    }
}

/// Recovery is also available when the original store cannot initialize.
pub fn recover_file(destination: &Path, source: &Path, modules: &[ModuleManifest]) -> Result<()> {
    let c = Connection::open_with_flags(source, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(db)?;
    let ok: String = c
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(db)?;
    if ok != "ok" {
        return Err(AppError::new("invalid_backup", ok));
    }
    for m in modules {
        let version: u32 = c
            .query_row(
                "SELECT version FROM framework_migrations WHERE owner=?",
                [&m.id],
                |r| r.get(0),
            )
            .map_err(db)?;
        if version != m.migration_version {
            return Err(AppError::new("backup_version", "备份模块版本与应用不匹配"));
        }
    }
    let staged = destination.with_extension("recovery-staged.sqlite");
    c.backup(rusqlite::DatabaseName::Main, &staged, None)
        .map_err(db)?;
    let suffix = uuid::Uuid::new_v4();
    for extension in ["", "-wal", "-shm"] {
        let path = PathBuf::from(format!("{}{extension}", destination.display()));
        if path.exists() {
            std::fs::rename(
                &path,
                PathBuf::from(format!("{}.recovery-{suffix}", path.display())),
            )
            .map_err(db)?;
        }
    }
    std::fs::rename(staged, destination).map_err(db)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn manifests() -> Vec<ModuleManifest> {
        vec![ModuleManifest {
            id: "tasks".into(),
            version: "1".into(),
            contract_version: 1,
            dependencies: vec![],
            optional_capabilities: vec![],
            commands: vec![],
            queries: vec![],
            events: vec![],
            pages: vec![],
            critical: true,
            migration_version: 1,
        }]
    }
    #[test]
    fn transactions_idempotency_recovery() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("app.sqlite");
        let s = Store::open(&path, &manifests()).unwrap();
        s.transact(Some(0), "one", |u| {
            u.repo("tasks")?.put("a", &42)?;
            u.emit("changed", &"a")?;
            Ok(42)
        })
        .unwrap();
        assert_eq!(s.transact(Some(0), "one", |_| Ok(99)).unwrap(), 42);
        assert!(s.transact(Some(0), "stale", |_| Ok(())).is_err());
        let failed: Result<()> = s.transact(Some(1), "fail", |u| {
            u.repo("tasks")?.put("a", &99)?;
            Err(AppError::new("fail", "fail"))
        });
        assert!(failed.is_err());
        assert_eq!(
            s.read(|u| u.repo("tasks")?.get::<i32>("a")).unwrap(),
            Some(42)
        );
        assert_eq!(s.pending_events().unwrap().len(), 1);
        let backup = d.path().join("backup.sqlite");
        s.backup(&backup).unwrap();
        drop(s);
        let s = Store::open(&path, &manifests()).unwrap();
        assert_eq!(s.pending_events().unwrap().len(), 1);
        s.restore(&backup, &manifests()).unwrap();
        assert_eq!(
            s.read(|u| u.repo("tasks")?.get::<i32>("a")).unwrap(),
            Some(42)
        );
    }
}
