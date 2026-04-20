use crate::data::db::{Database, unix_now};
use crate::error::AppError;

/// User profile stored in SQLite.
#[derive(Debug, Clone)]
pub struct Profile {
    pub id: i64,
    pub name: String,
    pub created_at: i64,
    pub github_id: Option<String>,
    pub github_username: Option<String>,
}

impl Database {
    /// Create a new profile with the given name.
    ///
    /// Terminal Drums is single-profile — there should only ever be one row in
    /// the `profile` table.  This method does not enforce uniqueness itself;
    /// callers should call `get_profile()` first.
    pub fn create_profile(&self, name: &str) -> Result<Profile, AppError> {
        let now = unix_now();
        self.conn.execute(
            "INSERT INTO profile (name, created_at) VALUES (?1, ?2)",
            rusqlite::params![name, now],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(Profile {
            id,
            name: name.to_string(),
            created_at: now,
            github_id: None,
            github_username: None,
        })
    }

    /// Return the first (and normally only) profile, or `None` if the table is empty.
    pub fn get_profile(&self) -> Result<Option<Profile>, AppError> {
        use rusqlite::OptionalExtension;

        let result = self
            .conn
            .query_row(
                "SELECT id, name, created_at, github_id, github_username
                 FROM profile
                 ORDER BY id ASC
                 LIMIT 1",
                [],
                |row| {
                    Ok(Profile {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        created_at: row.get(2)?,
                        github_id: row.get(3)?,
                        github_username: row.get(4)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// Delete all profiles, scores, and preferences. Used by /reset command.
    pub fn reset_all(&self) -> Result<(), AppError> {
        self.conn.execute_batch(
            "DELETE FROM scores; DELETE FROM preferences; DELETE FROM profile;",
        )?;
        Ok(())
    }

    /// Update the display name for the profile with the given id.
    pub fn update_profile_name(&self, id: i64, name: &str) -> Result<(), AppError> {
        self.conn.execute(
            "UPDATE profile SET name = ?1 WHERE id = ?2",
            rusqlite::params![name, id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::data::db::Database;

    #[test]
    fn test_create_and_get_profile() {
        let db = Database::open_in_memory().expect("in-memory db");

        // No profile yet.
        let p = db.get_profile().unwrap();
        assert!(p.is_none());

        // Create one.
        let created = db.create_profile("Alice").expect("create_profile");
        assert_eq!(created.name, "Alice");
        assert!(created.id > 0);
        assert!(created.github_id.is_none());

        // Retrieve it.
        let loaded = db.get_profile().unwrap().expect("profile should exist");
        assert_eq!(loaded.id, created.id);
        assert_eq!(loaded.name, "Alice");
    }

    #[test]
    fn test_update_profile_name() {
        let db = Database::open_in_memory().expect("in-memory db");

        let p = db.create_profile("Bob").unwrap();
        db.update_profile_name(p.id, "Robert").unwrap();

        let loaded = db.get_profile().unwrap().unwrap();
        assert_eq!(loaded.name, "Robert");
    }
}
