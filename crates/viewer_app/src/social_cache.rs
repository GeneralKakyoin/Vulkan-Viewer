use rusqlite::{Connection, params};
use viewer_core::{DirectImMessage, FriendEntry, SocialState};

#[derive(Debug, Clone)]
pub struct SocialCacheConfig {
    pub path: String,
    pub max_messages_per_thread: usize,
}

impl SocialCacheConfig {
    pub fn from_env() -> Self {
        let path = std::env::var("VIEWER_SOCIAL_CACHE_PATH")
            .unwrap_or_else(|_| String::from("logs/social_cache.db"));
        let max_messages_per_thread = std::env::var("VIEWER_SOCIAL_CACHE_MAX_MESSAGES_PER_THREAD")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(500)
            .max(50);
        Self {
            path,
            max_messages_per_thread,
        }
    }
}

pub struct SocialCache {
    conn: Connection,
    max_messages_per_thread: usize,
}

impl SocialCache {
    pub fn open(config: &SocialCacheConfig) -> Result<Self, String> {
        if let Some(parent) = std::path::Path::new(&config.path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(&config.path).map_err(|err| err.to_string())?;
        let mut cache = Self {
            conn,
            max_messages_per_thread: config.max_messages_per_thread,
        };
        cache.init_schema()?;
        Ok(cache)
    }

    fn init_schema(&mut self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS name_cache (
                    id TEXT PRIMARY KEY,
                    display_name TEXT NOT NULL,
                    source TEXT NOT NULL,
                    updated_unix_ms INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS im_messages (
                    db_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    dedupe_key TEXT NOT NULL UNIQUE,
                    session_id TEXT NOT NULL,
                    peer_id TEXT NOT NULL,
                    from_id TEXT NOT NULL,
                    from_name TEXT NOT NULL,
                    text TEXT NOT NULL,
                    observed_at_unix_ms INTEGER NOT NULL,
                    outgoing INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_im_messages_session_time
                    ON im_messages(session_id, observed_at_unix_ms, db_id);
                ",
            )
            .map_err(|err| err.to_string())
    }

    pub fn load_cached_social_state(&self) -> Result<SocialState, String> {
        let mut social = SocialState::default();

        let mut names_stmt = self
            .conn
            .prepare(
                "SELECT id, display_name, source, updated_unix_ms
                 FROM name_cache
                 ORDER BY updated_unix_ms ASC, id ASC",
            )
            .map_err(|err| err.to_string())?;
        let names = names_stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|err| err.to_string())?;
        for row in names {
            let (id, display_name, source, updated_unix_ms) = row.map_err(|err| err.to_string())?;
            social.upsert_friend(FriendEntry {
                id,
                display_name: Some(display_name),
                name_source: Some(source),
                last_name_resolved_unix_ms: Some(updated_unix_ms.max(0) as u64),
                online: false,
                rights_has: 0,
                rights_given: 0,
                last_changed_unix_ms: 0,
            });
        }

        let mut im_stmt = self
            .conn
            .prepare(
                "SELECT db_id, session_id, peer_id, from_id, from_name, text, observed_at_unix_ms, outgoing
                 FROM im_messages
                 ORDER BY observed_at_unix_ms ASC, db_id ASC",
            )
            .map_err(|err| err.to_string())?;
        let messages = im_stmt
            .query_map([], |row| {
                Ok(DirectImMessage {
                    id: row.get::<_, i64>(0)?.max(0) as u64,
                    session_id: row.get::<_, String>(1)?,
                    peer_id: row.get::<_, String>(2)?,
                    from_id: row.get::<_, String>(3)?,
                    from_name: row.get::<_, String>(4)?,
                    text: row.get::<_, String>(5)?,
                    observed_at_unix_ms: row.get::<_, i64>(6)?.max(0) as u64,
                    outgoing: row.get::<_, i64>(7)? != 0,
                })
            })
            .map_err(|err| err.to_string())?;
        for row in messages {
            let message = row.map_err(|err| err.to_string())?;
            let peer = message.peer_id.clone();
            social.upsert_thread_message(message, &peer);
        }

        Ok(social)
    }

    pub fn upsert_name_cache(
        &mut self,
        id: &str,
        display_name: &str,
        source: &str,
        updated_unix_ms: u64,
    ) -> Result<(), String> {
        if id.trim().is_empty() || display_name.trim().is_empty() {
            return Ok(());
        }
        self.conn
            .execute(
                "
                INSERT INTO name_cache(id, display_name, source, updated_unix_ms)
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(id) DO UPDATE SET
                    display_name=excluded.display_name,
                    source=excluded.source,
                    updated_unix_ms=excluded.updated_unix_ms
                WHERE excluded.updated_unix_ms >= name_cache.updated_unix_ms
                ",
                params![id, display_name, source, updated_unix_ms as i64],
            )
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn store_im_message(&mut self, message: &DirectImMessage) -> Result<(), String> {
        let dedupe_key = format!(
            "{}|{}|{}|{}|{}|{}",
            message.session_id,
            message.peer_id,
            message.from_id,
            message.observed_at_unix_ms,
            if message.outgoing { 1 } else { 0 },
            message.text
        );
        self.conn
            .execute(
                "
                INSERT OR IGNORE INTO im_messages(
                    dedupe_key, session_id, peer_id, from_id, from_name, text, observed_at_unix_ms, outgoing
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ",
                params![
                    dedupe_key,
                    message.session_id,
                    message.peer_id,
                    message.from_id,
                    message.from_name,
                    message.text,
                    message.observed_at_unix_ms as i64,
                    if message.outgoing { 1i64 } else { 0i64 }
                ],
            )
            .map_err(|err| err.to_string())?;
        self.prune_thread(&message.session_id)?;
        Ok(())
    }

    fn prune_thread(&mut self, session_id: &str) -> Result<(), String> {
        self.conn
            .execute(
                "
                DELETE FROM im_messages
                WHERE session_id = ?1
                  AND db_id NOT IN (
                      SELECT db_id
                      FROM im_messages
                      WHERE session_id = ?1
                      ORDER BY observed_at_unix_ms DESC, db_id DESC
                      LIMIT ?2
                  )
                ",
                params![session_id, self.max_messages_per_thread as i64],
            )
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db_path() -> String {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("/tmp/viewer_social_cache_test_{ts}.db")
    }

    #[test]
    fn schema_init_is_idempotent() {
        let config = SocialCacheConfig {
            path: temp_db_path(),
            max_messages_per_thread: 100,
        };
        let first = SocialCache::open(&config);
        assert!(first.is_ok());
        let second = SocialCache::open(&config);
        assert!(second.is_ok());
    }

    #[test]
    fn name_cache_newer_value_wins() {
        let config = SocialCacheConfig {
            path: temp_db_path(),
            max_messages_per_thread: 100,
        };
        let mut cache = SocialCache::open(&config).expect("cache should open");
        cache
            .upsert_name_cache("a", "Older", "test", 5)
            .expect("older insert should succeed");
        cache
            .upsert_name_cache("a", "Newest", "test", 10)
            .expect("newer insert should succeed");
        cache
            .upsert_name_cache("a", "Ignored", "test", 8)
            .expect("older overwrite should be ignored");
        let social = cache.load_cached_social_state().expect("state should load");
        let friend = social
            .friends
            .iter()
            .find(|f| f.id == "a")
            .expect("friend should exist");
        assert_eq!(friend.display_name.as_deref(), Some("Newest"));
    }

    #[test]
    fn im_dedupe_and_prune_keeps_recent_messages() {
        let config = SocialCacheConfig {
            path: temp_db_path(),
            max_messages_per_thread: 2,
        };
        let mut cache = SocialCache::open(&config).expect("cache should open");
        for idx in 0..4 {
            let message = DirectImMessage {
                id: idx,
                session_id: String::from("session"),
                peer_id: String::from("peer"),
                from_id: String::from("from"),
                from_name: String::from("name"),
                text: format!("m{idx}"),
                observed_at_unix_ms: idx,
                outgoing: false,
            };
            cache
                .store_im_message(&message)
                .expect("message should store");
        }
        let duplicate = DirectImMessage {
            id: 99,
            session_id: String::from("session"),
            peer_id: String::from("peer"),
            from_id: String::from("from"),
            from_name: String::from("name"),
            text: String::from("m3"),
            observed_at_unix_ms: 3,
            outgoing: false,
        };
        cache
            .store_im_message(&duplicate)
            .expect("duplicate should not fail");

        let social = cache.load_cached_social_state().expect("state should load");
        let thread = social
            .im_threads
            .iter()
            .find(|t| t.session_id == "session")
            .expect("thread should exist");
        assert_eq!(thread.messages.len(), 2);
        assert_eq!(thread.messages[0].text, "m2");
        assert_eq!(thread.messages[1].text, "m3");
    }
}
