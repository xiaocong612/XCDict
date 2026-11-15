//! dict_engine.rs — High-performance SQLite-backed dictionary query engine in Rust.
//!
//! Add to your Cargo.toml:
//! ```toml
//! [dependencies]
//! rusqlite = { version = "0.31", features = ["bundled"] }
//! serde = { version = "1.0", features = ["derive"] }
//! serde_json = "1.0"
//! ```

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Example {
    pub en: String,
    pub zh: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sense {
    pub pos: String,
    pub def_zh: String,
    pub def_en: Option<String>,
    pub examples: Vec<Example>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CultureNote {
    pub origin_lang: Option<String>,
    pub category: Option<String>,
    pub note_zh: Option<String>,
    pub note_en: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WordDetail {
    pub word: String,
    pub phonetic_us: String,
    pub phonetic_uk: String,
    pub tag: String,
    pub frequency: Option<i64>,
    pub senses: Vec<Sense>,
    pub culture: Option<CultureNote>,
}

pub struct XCDictEngine {
    conn: Connection,
}

impl XCDictEngine {
    /// Initialize with read-only connection to xcdict.db.
    pub fn open(db_path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_with_flags(
            db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        Ok(Self { conn })
    }

    /// Query full details of a headword (typically < 0.1 ms).
    pub fn query_word(&self, word: &str) -> Result<Option<WordDetail>, rusqlite::Error> {
        let clean_word = word.trim().to_lowercase();
        let mut stmt = self.conn.prepare(
            "SELECT id, word, phonetic_us, phonetic_uk, tag, frequency FROM words WHERE word = ? LIMIT 1"
        )?;

        let word_row = stmt.query_row([&clean_word], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<i64>>(5)?,
            ))
        });

        match word_row {
            Ok((wid, word, us, uk, tag, frequency)) => {
                // Fetch senses
                let mut s_stmt = self.conn.prepare(
                    "SELECT pos, def_zh, def_en, examples FROM senses WHERE word_id = ? ORDER BY id"
                )?;
                let sense_iter = s_stmt.query_map([wid], |row| {
                    let exs_str: String = row.get(3).unwrap_or_default();
                    let examples: Vec<Example> = serde_json::from_str(&exs_str).unwrap_or_default();
                    Ok(Sense {
                        pos: row.get(0).unwrap_or_default(),
                        def_zh: row.get(1).unwrap_or_default(),
                        def_en: row.get(2).ok(),
                        examples,
                    })
                })?;

                let mut senses = Vec::new();
                for s in sense_iter {
                    senses.push(s?);
                }

                // Fetch optional culture note
                let mut c_stmt = self.conn.prepare(
                    "SELECT origin_lang, category, note_zh, note_en FROM culture_notes WHERE word_id = ? LIMIT 1"
                )?;
                let culture = c_stmt.query_row([wid], |row| {
                    Ok(CultureNote {
                        origin_lang: row.get(0).ok(),
                        category: row.get(1).ok(),
                        note_zh: row.get(2).ok(),
                        note_en: row.get(3).ok(),
                    })
                }).ok();

                Ok(Some(WordDetail {
                    word,
                    phonetic_us: us,
                    phonetic_uk: uk,
                    tag,
                    frequency,
                    senses,
                    culture,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Autocomplete search suggestions by prefix (e.g., "app" -> "apple", "application", ...).
    pub fn search_suggestions(&self, prefix: &str, limit: usize) -> Result<Vec<String>, rusqlite::Error> {
        let pattern = format!("{}%", prefix.trim().to_lowercase());
        let mut stmt = self.conn.prepare(
            "SELECT word FROM words WHERE word LIKE ? ORDER BY length(word), word LIMIT ?"
        )?;
        let rows = stmt.query_map([&pattern, &limit.to_string()], |row| row.get(0))?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// List words filtered by examination tag (e.g., "gk", "zk", "cet4", "kaoyan").
    pub fn list_by_tag(&self, tag: &str, limit: usize, offset: usize) -> Result<Vec<String>, rusqlite::Error> {
        let pattern = format!("%{}%", tag.trim().to_lowercase());
        let mut stmt = self.conn.prepare(
            "SELECT word FROM words WHERE tag LIKE ? ORDER BY id LIMIT ? OFFSET ?"
        )?;
        let rows = stmt.query_map([&pattern, &limit.to_string(), &offset.to_string()], |row| row.get(0))?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }
}
