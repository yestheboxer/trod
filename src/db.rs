use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::Path;

pub struct Database {
    conn: Connection,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub path: String,
    pub visit_count: i64,
    pub last_visited: DateTime<Utc>,
    pub first_visited: DateTime<Utc>,
}

#[derive(Debug)]
pub struct Stats {
    pub total_directories: i64,
    pub total_visits: i64,
    pub most_visited: Option<DirEntry>,
    pub oldest_entry: Option<DirEntry>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_default() -> Result<Self> {
        let data_dir = dirs::data_dir()
            .context("Could not determine data directory")?
            .join("trod");
        std::fs::create_dir_all(&data_dir)?;
        Self::open(&data_dir.join("history.db"))
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS directories (
                id INTEGER PRIMARY KEY,
                path TEXT UNIQUE NOT NULL,
                visit_count INTEGER DEFAULT 1,
                last_visited TEXT NOT NULL,
                first_visited TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS visits (
                id INTEGER PRIMARY KEY,
                directory_id INTEGER NOT NULL REFERENCES directories(id) ON DELETE CASCADE,
                timestamp TEXT NOT NULL,
                session_id TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_directories_last_visited ON directories(last_visited);
            CREATE INDEX IF NOT EXISTS idx_directories_path ON directories(path);
            CREATE INDEX IF NOT EXISTS idx_visits_timestamp ON visits(timestamp);",
        )?;
        Ok(())
    }

    pub fn add(&self, path: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO directories (path, visit_count, last_visited, first_visited)
             VALUES (?1, 1, ?2, ?2)
             ON CONFLICT(path) DO UPDATE SET
                visit_count = visit_count + 1,
                last_visited = ?2",
            params![path, now],
        )?;
        // Also record in visits table
        let dir_id: i64 = self.conn.query_row(
            "SELECT id FROM directories WHERE path = ?1",
            params![path],
            |row| row.get(0),
        )?;
        self.conn.execute(
            "INSERT INTO visits (directory_id, timestamp) VALUES (?1, ?2)",
            params![dir_id, now],
        )?;
        Ok(())
    }

    pub fn list_recent(&self, limit: usize) -> Result<Vec<DirEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT path, visit_count, last_visited, first_visited
             FROM directories
             ORDER BY last_visited DESC
             LIMIT ?1",
        )?;
        let entries = stmt
            .query_map(params![limit as i64], |row| {
                Ok(DirEntry {
                    path: row.get(0)?,
                    visit_count: row.get(1)?,
                    last_visited: row
                        .get::<_, String>(2)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    first_visited: row
                        .get::<_, String>(3)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    pub fn list_frequent(&self, limit: usize) -> Result<Vec<DirEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT path, visit_count, last_visited, first_visited
             FROM directories
             ORDER BY visit_count DESC, last_visited DESC
             LIMIT ?1",
        )?;
        let entries = stmt
            .query_map(params![limit as i64], |row| {
                Ok(DirEntry {
                    path: row.get(0)?,
                    visit_count: row.get(1)?,
                    last_visited: row
                        .get::<_, String>(2)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    first_visited: row
                        .get::<_, String>(3)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    pub fn forget(&self, path: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM visits WHERE directory_id IN (SELECT id FROM directories WHERE path = ?1)",
            params![path],
        )?;
        self.conn
            .execute("DELETE FROM directories WHERE path = ?1", params![path])?;
        Ok(())
    }

    pub fn clean(&self) -> Result<usize> {
        let paths: Vec<String> = {
            let mut stmt = self.conn.prepare("SELECT path FROM directories")?;
            let result = stmt.query_map([], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?;
            result
        };

        let mut removed = 0;
        for path in paths {
            if !Path::new(&path).exists() {
                self.forget(&path)?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    pub fn stats(&self) -> Result<Stats> {
        let total_directories: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM directories", [], |row| row.get(0))?;
        let total_visits: i64 = self
            .conn
            .query_row("SELECT COALESCE(SUM(visit_count), 0) FROM directories", [], |row| {
                row.get(0)
            })?;
        let most_visited = self.list_frequent(1)?.into_iter().next();
        let oldest_entry = {
            let mut stmt = self.conn.prepare(
                "SELECT path, visit_count, last_visited, first_visited
                 FROM directories ORDER BY first_visited ASC LIMIT 1",
            )?;
            let result = stmt.query_map([], |row| {
                Ok(DirEntry {
                    path: row.get(0)?,
                    visit_count: row.get(1)?,
                    last_visited: row
                        .get::<_, String>(2)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    first_visited: row
                        .get::<_, String>(3)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
            result.into_iter().next()
        };
        Ok(Stats {
            total_directories,
            total_visits,
            most_visited,
            oldest_entry,
        })
    }

    pub fn back(&self, n: usize) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT path FROM directories ORDER BY last_visited DESC LIMIT 1 OFFSET ?1",
        )?;
        let result = stmt
            .query_map(params![n as i64], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(result.into_iter().next())
    }

    pub fn all_entries(&self) -> Result<Vec<DirEntry>> {
        self.list_recent(10000)
    }
}
