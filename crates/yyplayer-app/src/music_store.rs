//! SQLite music repository. Connections belong to disk workers, never the UI/Engine.
use player_core::{
    library::{LibrarySettings, Song},
    playlists::{Playlist, Playlists},
    settings::Settings,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

const APPLICATION_ID: i32 = 0x5959504c;
const SCHEMA: i32 = 1;
const MAX_SONGS: usize = 20_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Collections {
    pub library: LibrarySettings,
    pub playlists: Playlists,
}

#[derive(Debug)]
pub struct MusicStore {
    connection: Connection,
}

pub fn database_path(settings: &Path) -> PathBuf {
    settings.with_extension("library.sqlite3")
}

fn sql_error(error: rusqlite::Error) -> String {
    use rusqlite::ErrorCode;
    let category = match error.sqlite_error_code() {
        Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked) => "忙，请稍后重试",
        Some(ErrorCode::ReadOnly | ErrorCode::CannotOpen) => "路径不可写或无法打开",
        Some(ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase) => "损坏，原库保留",
        Some(ErrorCode::ConstraintViolation) => "记录约束冲突",
        _ => "操作失败",
    };
    format!("音乐数据库{category}：{error}")
}

fn sql_integer(value: u64) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| "音乐数据库整数超出范围；原记录保留".into())
}
fn unsigned_integer(value: i64) -> Result<u64, String> {
    u64::try_from(value).map_err(|_| "音乐数据库整数损坏；原记录保留".into())
}

// Path identity is native bytes / UTF-16, never a lossy display string.
fn encode_path(path: &Path) -> Result<Vec<u8>, String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let units: Vec<_> = path.as_os_str().encode_wide().collect();
        if units.is_empty() || units.contains(&0) {
            return Err("音乐数据库路径为空或含 NUL".into());
        }
        Ok(std::iter::once(b'W')
            .chain(units.into_iter().flat_map(u16::to_le_bytes))
            .collect())
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let bytes = path.as_os_str().as_bytes();
        if bytes.is_empty() || bytes.contains(&0) {
            return Err("音乐数据库路径为空或含 NUL".into());
        }
        Ok(std::iter::once(b'U').chain(bytes.iter().copied()).collect())
    }
}

fn decode_path(bytes: Vec<u8>) -> Result<PathBuf, String> {
    #[cfg(windows)]
    if bytes.first() == Some(&b'W') && bytes.len() > 1 && (bytes.len() - 1).is_multiple_of(2) {
        use std::os::windows::ffi::OsStringExt;
        let units: Vec<_> = bytes[1..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|v| u16::from_le_bytes([v[0], v[1]]))
            .collect();
        if !units.contains(&0) {
            return Ok(std::ffi::OsString::from_wide(&units).into());
        }
    }
    #[cfg(unix)]
    if bytes.first() == Some(&b'U') && bytes.len() > 1 && !bytes[1..].contains(&0) {
        use std::os::unix::ffi::OsStringExt;
        return Ok(std::ffi::OsString::from_vec(bytes[1..].to_vec()).into());
    }
    Err("音乐数据库路径编码损坏或来自其他平台；原库保留".into())
}

impl MusicStore {
    pub fn open(settings_path: &Path, legacy: &Settings) -> Result<Self, String> {
        if legacy.version == 2 && settings_path.exists() && !database_path(settings_path).exists() {
            return Err(
                "音乐数据库缺失，设置和迁移备份保留；请恢复 settings.library.sqlite3 后点击重扫"
                    .into(),
            );
        }
        Self::open_at(
            &database_path(settings_path),
            &settings_path.with_extension("library.json"),
            legacy,
            Some(settings_path),
        )
    }

    pub fn open_index(cache: &Path, library: LibrarySettings) -> Result<Self, String> {
        Self::open_at(
            &cache.with_extension("sqlite3"),
            cache,
            &Settings {
                library,
                ..Default::default()
            },
            None,
        )
    }

    fn open_at(
        database: &Path,
        cache: &Path,
        legacy: &Settings,
        backup: Option<&Path>,
    ) -> Result<Self, String> {
        if let Some(parent) = database.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| format!("音乐数据库目录：{e}"))?;
        }
        let mut connection = Connection::open(database).map_err(sql_error)?;
        connection
            .busy_timeout(Duration::from_secs(2))
            .map_err(sql_error)?;
        connection
            .pragma_update(None, "foreign_keys", true)
            .map_err(sql_error)?;
        let check: String = connection
            .query_row("PRAGMA quick_check(1)", [], |r| r.get(0))
            .map_err(sql_error)?;
        if check != "ok" {
            return Err(format!("音乐数据库检查失败，原库保留：{check}"));
        }
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql_error)?;
        let version: i32 = tx
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(sql_error)?;
        let application: i32 = tx
            .pragma_query_value(None, "application_id", |r| r.get(0))
            .map_err(sql_error)?;
        if version == 0 && application == 0 {
            if legacy.version == 2 && backup.is_some_and(Path::exists) {
                return Err("音乐数据库未初始化，原文件保留；请恢复原库或迁移备份后重试".into());
            }
            let tables: i64 = tx.query_row("SELECT count(*) FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%'", [], |r| r.get(0)).map_err(sql_error)?;
            if tables != 0 {
                return Err("音乐数据库不是 YYPlayer 数据库；原库保留".into());
            }
            legacy.library.validate()?;
            legacy.playlists.validate()?;
            let songs = read_legacy_index(cache)?;
            // Back up source bytes before committing the migration. Never delete/overwrite them.
            if let Some(path) = backup.filter(|p| p.exists()) {
                let bytes = std::fs::read(path).map_err(|e| format!("迁移备份读取失败：{e}"))?;
                let target = path.with_extension("pre-sqlite.json");
                if !target.exists() {
                    super::atomic_bytes(&target, &bytes)?;
                }
            }
            tx.execute_batch(
                "CREATE TABLE metadata(key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 CREATE TABLE tracks(
                   id INTEGER PRIMARY KEY, path BLOB NOT NULL UNIQUE CHECK(length(path)>1),
                   title TEXT NOT NULL, artist TEXT NOT NULL, album TEXT NOT NULL,
                   artist_key TEXT NOT NULL, album_key TEXT NOT NULL,
                   seconds INTEGER NOT NULL CHECK(seconds>=0),
                   indexed INTEGER NOT NULL CHECK(indexed IN (0,1)));
                 CREATE INDEX tracks_artist ON tracks(indexed,artist_key,id);
                 CREATE INDEX tracks_album ON tracks(indexed,artist_key,album_key,id);
                 CREATE TABLE library_sources(
                   kind INTEGER NOT NULL CHECK(kind BETWEEN 0 AND 2),
                   position INTEGER NOT NULL CHECK(position>=0), path BLOB NOT NULL,
                   PRIMARY KEY(kind,position), UNIQUE(kind,path));
                 CREATE TABLE playlists(
                   id INTEGER PRIMARY KEY CHECK(id>0), name TEXT NOT NULL,
                   name_key TEXT NOT NULL UNIQUE, position INTEGER NOT NULL UNIQUE CHECK(position>=0));
                 CREATE TABLE playlist_tracks(
                   playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
                   track_id INTEGER NOT NULL REFERENCES tracks(id),
                   position INTEGER NOT NULL CHECK(position>=0),
                   PRIMARY KEY(playlist_id,track_id), UNIQUE(playlist_id,position));
                 CREATE INDEX playlist_tracks_track ON playlist_tracks(track_id);"
            ).map_err(sql_error)?;
            write_sources(&tx, &legacy.library)?;
            write_playlists(&tx, &legacy.playlists, None)?;
            for song in &songs {
                upsert_song(&tx, song)?;
            }
            tx.pragma_update(None, "application_id", APPLICATION_ID)
                .map_err(sql_error)?;
            tx.pragma_update(None, "user_version", SCHEMA)
                .map_err(sql_error)?;
            tx.execute(
                "INSERT INTO metadata VALUES ('path_encoding',?1)",
                [if cfg!(windows) {
                    "windows-utf16le"
                } else {
                    "unix-bytes"
                }],
            )
            .map_err(sql_error)?;
        } else if version != SCHEMA || application != APPLICATION_ID {
            return Err(format!(
                "不支持音乐数据库版本 / 类型 {version}/{application}；原库保留"
            ));
        }
        tx.commit().map_err(sql_error)?;
        let store = Self { connection };
        let encoding: Option<String> = store
            .connection
            .query_row(
                "SELECT value FROM metadata WHERE key='path_encoding'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(sql_error)?;
        if encoding.as_deref()
            != Some(if cfg!(windows) {
                "windows-utf16le"
            } else {
                "unix-bytes"
            })
        {
            return Err("音乐数据库路径编码版本或平台不匹配；原库保留".into());
        }
        // Validate all collection records on reopen, including broken foreign keys.
        let broken: bool = store
            .connection
            .prepare("PRAGMA foreign_key_check")
            .map_err(sql_error)?
            .query([])
            .map_err(sql_error)?
            .next()
            .map_err(sql_error)?
            .is_some();
        if broken {
            return Err("音乐数据库关联损坏；原库保留".into());
        }
        store.collections()?;
        Ok(store)
    }

    pub fn collections(&self) -> Result<Collections, String> {
        let mut library = LibrarySettings::default();
        let mut stmt = self
            .connection
            .prepare("SELECT kind,path FROM library_sources ORDER BY kind,position LIMIT 20129")
            .map_err(sql_error)?;
        for item in stmt
            .query_map([], |r| Ok((r.get::<_, i32>(0)?, r.get::<_, Vec<u8>>(1)?)))
            .map_err(sql_error)?
        {
            let (kind, bytes) = item.map_err(sql_error)?;
            let paths = match kind {
                0 => &mut library.roots,
                1 => &mut library.files,
                2 => &mut library.excluded,
                _ => return Err("音乐数据库目录分类损坏".into()),
            };
            paths.push(decode_path(bytes)?);
        }
        library.validate()?;
        let mut playlists = Playlists {
            version: 1,
            next_id: self
                .connection
                .query_row(
                    "SELECT value FROM metadata WHERE key='next_playlist_id'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .map_err(sql_error)?
                .parse()
                .map_err(|_| "歌单身份序列损坏")?,
            lists: vec![],
        };
        let mut stmt = self
            .connection
            .prepare("SELECT id,name FROM playlists ORDER BY position LIMIT 129")
            .map_err(sql_error)?;
        for list in stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(sql_error)?
        {
            let (id, name) = list.map_err(sql_error)?;
            let mut tracks = self.connection.prepare("SELECT t.path FROM playlist_tracks p JOIN tracks t ON t.id=p.track_id WHERE p.playlist_id=?1 ORDER BY p.position LIMIT 20001").map_err(sql_error)?;
            let paths = tracks
                .query_map([id], |r| r.get::<_, Vec<u8>>(0))
                .map_err(sql_error)?
                .map(|r| decode_path(r.map_err(sql_error)?))
                .collect::<Result<Vec<_>, String>>()?;
            playlists.lists.push(Playlist {
                id: unsigned_integer(id)?,
                name,
                tracks: paths,
            });
            if playlists
                .lists
                .iter()
                .map(|p| p.tracks.len())
                .sum::<usize>()
                > MAX_SONGS
            {
                return Err("音乐数据库歌单成员超过 20000 首".into());
            }
        }
        playlists.validate()?;
        Ok(Collections { library, playlists })
    }

    pub fn save_collections(&mut self, value: &Collections) -> Result<(), String> {
        value.library.validate()?;
        value.playlists.validate()?;
        let prior = self.collections()?;
        if &prior == value {
            return Ok(());
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql_error)?;
        if prior.library != value.library {
            write_sources(&tx, &value.library)?;
        }
        if prior.playlists != value.playlists {
            write_playlists(&tx, &value.playlists, Some(&prior.playlists))?;
        }
        tx.commit().map_err(sql_error)
    }

    pub fn songs(&self) -> Result<Vec<Song>, String> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT path,title,artist,album,seconds FROM tracks WHERE indexed=1 LIMIT 20001",
            )
            .map_err(sql_error)?;
        let mut songs = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, Vec<u8>>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, i64>(4)?,
                ))
            })
            .map_err(sql_error)?
            .map(|r| {
                let (bytes, title, artist, album, seconds) = r.map_err(sql_error)?;
                Ok(Song {
                    path: decode_path(bytes)?,
                    title,
                    artist,
                    album,
                    seconds: unsigned_integer(seconds)?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        if songs.len() > MAX_SONGS {
            return Err("音乐数据库索引超过 20000 首".into());
        }
        songs.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(songs)
    }

    pub fn replace_index(
        &mut self,
        songs: &[Song],
        generation: &AtomicU64,
        expected: u64,
    ) -> Result<(), String> {
        if songs.len() > MAX_SONGS {
            return Err("音乐数据库索引超过 20000 首".into());
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql_error)?;
        tx.execute_batch("CREATE TEMP TABLE IF NOT EXISTS scanned_tracks(path BLOB PRIMARY KEY); DELETE FROM scanned_tracks;").map_err(sql_error)?;
        for song in songs {
            if generation.load(Ordering::Relaxed) != expected {
                return Err("音乐扫描已取消；已提交索引保留".into());
            }
            upsert_song(&tx, song)?;
            tx.execute(
                "INSERT INTO scanned_tracks VALUES (?1)",
                [encode_path(&song.path)?],
            )
            .map_err(sql_error)?;
        }
        if generation.load(Ordering::Relaxed) != expected {
            return Err("音乐扫描已取消；已提交索引保留".into());
        }
        tx.execute("UPDATE tracks SET indexed=0 WHERE indexed=1 AND NOT EXISTS(SELECT 1 FROM scanned_tracks s WHERE s.path=tracks.path)", []).map_err(sql_error)?;
        if generation.load(Ordering::Relaxed) != expected {
            return Err("音乐扫描已取消；已提交索引保留".into());
        }
        tx.commit().map_err(sql_error)
    }
}

fn write_sources(tx: &rusqlite::Transaction<'_>, library: &LibrarySettings) -> Result<(), String> {
    tx.execute("DELETE FROM library_sources", [])
        .map_err(sql_error)?;
    let mut stmt = tx
        .prepare("INSERT INTO library_sources VALUES (?1,?2,?3)")
        .map_err(sql_error)?;
    for (kind, paths) in [&library.roots, &library.files, &library.excluded]
        .iter()
        .enumerate()
    {
        for (position, path) in paths.iter().enumerate() {
            stmt.execute(params![kind as i32, position as i64, encode_path(path)?])
                .map_err(sql_error)?;
        }
    }
    Ok(())
}

fn write_playlists(
    tx: &rusqlite::Transaction<'_>,
    playlists: &Playlists,
    prior: Option<&Playlists>,
) -> Result<(), String> {
    // Temporary unique keys allow atomic position / name swaps. Control chars
    // cannot be user names. Readers only see the state before or after commit.
    tx.execute(
        "UPDATE playlists SET position=position+128,name_key=char(0)||cast(id AS TEXT)",
        [],
    )
    .map_err(sql_error)?;
    if let Some(prior) = prior {
        for removed in &prior.lists {
            if !playlists.lists.iter().any(|p| p.id == removed.id) {
                tx.execute(
                    "DELETE FROM playlists WHERE id=?1",
                    [sql_integer(removed.id)?],
                )
                .map_err(sql_error)?;
            }
        }
    }
    let mut insert = tx
        .prepare("INSERT INTO playlists VALUES (?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET name=excluded.name,name_key=excluded.name_key,position=excluded.position")
        .map_err(sql_error)?;
    let mut track = tx.prepare("INSERT INTO tracks(path,title,artist,album,artist_key,album_key,seconds,indexed) VALUES (?1,'','','','','',0,0) ON CONFLICT(path) DO NOTHING").map_err(sql_error)?;
    let mut member = tx
        .prepare("INSERT INTO playlist_tracks SELECT ?1,id,?2 FROM tracks WHERE path=?3")
        .map_err(sql_error)?;
    for (position, list) in playlists.lists.iter().enumerate() {
        let id = sql_integer(list.id)?;
        insert
            .execute(params![
                id,
                list.name,
                list.name.trim().to_lowercase(),
                position as i64
            ])
            .map_err(sql_error)?;
        if prior
            .and_then(|p| p.lists.iter().find(|p| p.id == list.id))
            .is_some_and(|p| p.tracks == list.tracks)
        {
            continue;
        }
        tx.execute("DELETE FROM playlist_tracks WHERE playlist_id=?1", [id])
            .map_err(sql_error)?;
        for (position, path) in list.tracks.iter().enumerate() {
            let path = encode_path(path)?;
            track.execute([&path]).map_err(sql_error)?;
            member
                .execute(params![id, position as i64, path])
                .map_err(sql_error)?;
        }
    }
    tx.execute("INSERT INTO metadata VALUES ('next_playlist_id',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [playlists.next_id.to_string()]).map_err(sql_error)?;
    Ok(())
}

fn upsert_song(tx: &rusqlite::Transaction<'_>, song: &Song) -> Result<(), String> {
    tx.execute("INSERT INTO tracks(path,title,artist,album,artist_key,album_key,seconds,indexed) VALUES (?1,?2,?3,?4,?5,?6,?7,1) ON CONFLICT(path) DO UPDATE SET title=excluded.title,artist=excluded.artist,album=excluded.album,artist_key=excluded.artist_key,album_key=excluded.album_key,seconds=excluded.seconds,indexed=1 WHERE tracks.indexed<>1 OR tracks.title<>excluded.title OR tracks.artist<>excluded.artist OR tracks.album<>excluded.album OR tracks.seconds<>excluded.seconds",
        params![encode_path(&song.path)?,song.title,song.artist,song.album,song.artist.trim().to_lowercase(),song.album.trim().to_lowercase(),sql_integer(song.seconds)?]).map_err(sql_error)?;
    Ok(())
}

fn read_legacy_index(path: &Path) -> Result<Vec<Song>, String> {
    if !path.exists() {
        return Ok(vec![]);
    }
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 32_000_000 {
        return Err("旧音乐索引超过 32MB；原文件保留".into());
    }
    #[derive(Deserialize)]
    struct Legacy {
        version: u32,
        #[serde(default)]
        video: bool,
        songs: Vec<Song>,
    }
    let cache: Legacy = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("旧音乐索引读取失败，原文件保留：{e}"))?;
    if cache.version != 1 || cache.video || cache.songs.len() > MAX_SONGS {
        return Err("旧音乐索引版本或容量无效；原文件保留".into());
    }
    let unique: std::collections::BTreeSet<_> = cache.songs.iter().map(|s| &s.path).collect();
    if unique.len() != cache.songs.len() {
        return Err("旧音乐索引路径重复；原文件保留".into());
    }
    Ok(cache.songs)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static ID: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "yyplayer-sqlite-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                ID.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn settings(&self) -> PathBuf {
            self.0.join("settings.json")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            for file in std::fs::read_dir(&self.0).unwrap() {
                std::fs::remove_file(file.unwrap().path()).unwrap();
            }
            std::fs::remove_dir(&self.0).unwrap();
        }
    }
    fn song(path: &str, artist: &str) -> Song {
        Song {
            path: path.into(),
            title: "歌曲 ' ; 中文".into(),
            artist: artist.into(),
            album: "同名专辑".into(),
            seconds: 42,
        }
    }
    fn legacy(f: &Fixture) -> Settings {
        let mut value = Settings {
            version: 1,
            ..Default::default()
        };
        value.library.roots.push("音乐目录".into());
        let id = value.playlists.create("夜航").unwrap();
        value
            .playlists
            .add(
                id,
                vec![
                    "音乐目录/b.flac".into(),
                    "失联目录/offline.flac".into(),
                    "音乐目录/a.flac".into(),
                ],
            )
            .unwrap();
        value
            .playlists
            .add(1, vec!["音乐目录/b.flac".into()])
            .unwrap();
        let bytes = serde_json::to_vec(&value).unwrap();
        std::fs::write(f.settings(), &bytes).unwrap();
        std::fs::write(f.settings().with_extension("library.json"),serde_json::to_vec(&serde_json::json!({"version":1,"config":value.library,"songs":[song("音乐目录/a.flac","歌手甲"),song("音乐目录/b.flac","歌手乙")]})).unwrap()).unwrap();
        value
    }
    #[test]
    fn legacy_migration_is_relational_lossless_and_idempotent() {
        let f = Fixture::new();
        let value = legacy(&f);
        let original = std::fs::read(f.settings()).unwrap();
        let cache = std::fs::read(f.settings().with_extension("library.json")).unwrap();
        let store = MusicStore::open(&f.settings(), &value).unwrap();
        assert_eq!(
            store.collections().unwrap(),
            Collections {
                library: value.library.clone(),
                playlists: value.playlists.clone()
            }
        );
        assert_eq!(store.songs().unwrap().len(), 2);
        let ids: Vec<i64> = store
            .connection
            .prepare("SELECT id FROM tracks ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            std::fs::read(f.settings().with_extension("pre-sqlite.json")).unwrap(),
            original
        );
        assert_eq!(std::fs::read(f.settings()).unwrap(), original);
        assert_eq!(
            std::fs::read(f.settings().with_extension("library.json")).unwrap(),
            cache
        );
        assert_eq!(
            store
                .connection
                .query_row(
                    "SELECT count(DISTINCT artist_key) FROM tracks WHERE indexed=1",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            2
        );
        drop(store);
        std::fs::write(
            f.settings().with_extension("library.json"),
            b"{bad old cache",
        )
        .unwrap();
        let store = MusicStore::open(&f.settings(), &Settings::default()).unwrap();
        assert_eq!(store.collections().unwrap().playlists, value.playlists);
        let reopened: Vec<i64> = store
            .connection
            .prepare("SELECT id FROM tracks ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(ids, reopened);
    }
    #[test]
    fn rescans_keep_ids_and_playlist_members_after_removal() {
        let f = Fixture::new();
        let value = legacy(&f);
        let mut store = MusicStore::open(&f.settings(), &value).unwrap();
        let id: i64 = store
            .connection
            .query_row(
                "SELECT id FROM tracks WHERE path=?1",
                [encode_path(Path::new("音乐目录/a.flac")).unwrap()],
                |r| r.get(0),
            )
            .unwrap();
        let generation = AtomicU64::new(4);
        store
            .replace_index(&[song("音乐目录/a.flac", "更新歌手")], &generation, 4)
            .unwrap();
        assert_eq!(store.songs().unwrap()[0].artist, "更新歌手");
        assert_eq!(store.collections().unwrap().playlists, value.playlists);
        assert_eq!(
            store
                .connection
                .query_row(
                    "SELECT id FROM tracks WHERE path=?1",
                    [encode_path(Path::new("音乐目录/a.flac")).unwrap()],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            id
        );
        store.replace_index(&[], &generation, 4).unwrap();
        assert!(store.songs().unwrap().is_empty());
        assert_eq!(store.collections().unwrap().playlists, value.playlists);
    }
    #[test]
    fn cancellation_and_invalid_batch_roll_back_the_whole_scan() {
        let f = Fixture::new();
        let value = legacy(&f);
        let mut store = MusicStore::open(&f.settings(), &value).unwrap();
        let generation = AtomicU64::new(8);
        assert!(store.replace_index(&[], &generation, 7).is_err());
        let mut invalid = song("invalid.flac", "歌手");
        invalid.seconds = u64::MAX;
        assert!(
            store
                .replace_index(&[song("new.flac", "新歌手"), invalid], &generation, 8)
                .is_err()
        );
        assert_eq!(store.songs().unwrap().len(), 2);
        assert_eq!(store.collections().unwrap().playlists, value.playlists);
    }
    #[test]
    fn invalid_collections_and_constraints_preserve_prior_data() {
        let f = Fixture::new();
        let value = legacy(&f);
        let mut store = MusicStore::open(&f.settings(), &value).unwrap();
        let prior = store.collections().unwrap();
        let mut changed = prior.clone();
        changed.library.roots.push("音乐目录".into());
        changed.playlists.rename(2, "更名").unwrap();
        assert!(store.save_collections(&changed).is_err());
        assert_eq!(store.collections().unwrap(), prior);
        changed = prior.clone();
        changed.playlists.next_id = 0;
        assert!(store.save_collections(&changed).is_err());
        assert_eq!(store.collections().unwrap(), prior);
        assert!(
            store
                .connection
                .execute("INSERT INTO playlist_tracks VALUES (999,999,0)", [])
                .is_err()
        );
    }
    #[test]
    fn busy_writes_report_error_without_losing_the_old_catalog() {
        let f = Fixture::new();
        let value = legacy(&f);
        let mut store = MusicStore::open(&f.settings(), &value).unwrap();
        store
            .connection
            .busy_timeout(Duration::from_millis(10))
            .unwrap();
        let lock = Connection::open(database_path(&f.settings())).unwrap();
        lock.execute_batch("BEGIN IMMEDIATE").unwrap();
        let mut changed = store.collections().unwrap();
        changed.playlists.rename(2, "新名").unwrap();
        assert!(store.save_collections(&changed).unwrap_err().contains("忙"));
        lock.execute_batch("ROLLBACK").unwrap();
        assert_eq!(store.collections().unwrap().playlists, value.playlists);
        store.save_collections(&changed).unwrap();
        assert_eq!(store.collections().unwrap(), changed);
        store
            .connection
            .pragma_update(None, "query_only", true)
            .unwrap();
        let mut refused = changed.clone();
        refused.playlists.rename(2, "只读写入").unwrap();
        assert!(
            store
                .save_collections(&refused)
                .unwrap_err()
                .contains("不可写")
        );
        assert_eq!(store.collections().unwrap(), changed);
    }
    #[test]
    fn rename_and_reorder_keep_members_and_atomic_name_swaps() {
        let f = Fixture::new();
        let value = legacy(&f);
        let mut store = MusicStore::open(&f.settings(), &value).unwrap();
        let mut changed = store.collections().unwrap();
        let third = changed.playlists.create("第三歌单").unwrap();
        changed
            .playlists
            .add(third, vec!["音乐目录/a.flac".into()])
            .unwrap();
        store.save_collections(&changed).unwrap();
        let member_rows = |store: &MusicStore| -> Vec<i64> {
            store
                .connection
                .prepare("SELECT rowid FROM playlist_tracks ORDER BY rowid")
                .unwrap()
                .query_map([], |r| r.get(0))
                .unwrap()
                .map(Result::unwrap)
                .collect()
        };
        let before = member_rows(&store);
        let a = changed.playlists.lists[1].name.clone();
        let b = changed.playlists.lists[2].name.clone();
        changed.playlists.lists[1].name = b;
        changed.playlists.lists[2].name = a;
        changed.playlists.lists.swap(1, 2);
        store.save_collections(&changed).unwrap();
        assert_eq!(store.collections().unwrap(), changed);
        assert_eq!(member_rows(&store), before);
        changed.playlists.delete(third).unwrap();
        store.save_collections(&changed).unwrap();
        assert_eq!(store.collections().unwrap(), changed);
        assert_eq!(
            store
                .connection
                .query_row(
                    "SELECT count(*) FROM playlist_tracks WHERE playlist_id=?1",
                    [sql_integer(third).unwrap()],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
    #[test]
    fn corrupt_future_and_foreign_databases_are_never_replaced() {
        for mode in 0..3 {
            let f = Fixture::new();
            let value = legacy(&f);
            let database = database_path(&f.settings());
            if mode == 0 {
                std::fs::write(&database, b"damaged sqlite file").unwrap();
            } else {
                let connection = Connection::open(&database).unwrap();
                if mode == 1 {
                    connection
                        .pragma_update(None, "application_id", APPLICATION_ID)
                        .unwrap();
                    connection
                        .pragma_update(None, "user_version", SCHEMA + 1)
                        .unwrap();
                } else {
                    connection
                        .execute_batch("CREATE TABLE unrelated(value TEXT)")
                        .unwrap();
                }
            }
            let before = std::fs::read(&database).unwrap();
            assert!(MusicStore::open(&f.settings(), &value).is_err());
            assert_eq!(std::fs::read(&database).unwrap(), before);
            assert!(super::super::load(&f.settings()).1.is_empty());
        }
    }
    #[test]
    fn missing_database_after_migration_is_visible_and_does_not_reset_playlists() {
        let f = Fixture::new();
        let value = legacy(&f);
        super::super::atomic_save(&f.settings(), &value).unwrap();
        let loaded = super::super::load(&f.settings()).0;
        assert_eq!(loaded.version, 2);
        let bytes = std::fs::read(f.settings()).unwrap();
        std::fs::rename(database_path(&f.settings()), f.0.join("saved.sqlite3")).unwrap();
        assert!(
            MusicStore::open(&f.settings(), &loaded)
                .unwrap_err()
                .contains("缺失")
        );
        assert_eq!(std::fs::read(f.settings()).unwrap(), bytes);
        assert!(!database_path(&f.settings()).exists());
    }
    #[test]
    fn native_paths_roundtrip_without_unicode_substitution() {
        #[cfg(windows)]
        let path = {
            use std::os::windows::ffi::OsStringExt;
            PathBuf::from(std::ffi::OsString::from_wide(&[
                b'a' as u16,
                0xd800,
                b'b' as u16,
            ]))
        };
        #[cfg(unix)]
        let path = {
            use std::os::unix::ffi::OsStringExt;
            PathBuf::from(std::ffi::OsString::from_vec(vec![b'a', 0xff, b'b']))
        };
        assert_eq!(decode_path(encode_path(&path).unwrap()).unwrap(), path);
        assert!(decode_path(vec![b'W', 0]).is_err());
        assert!(encode_path(Path::new("")).is_err());
        let f = Fixture::new();
        let mut value = Settings::default();
        value.playlists.add(1, vec![path.clone()]).unwrap();
        super::super::atomic_save(&f.settings(), &value).unwrap();
        let saved = super::super::load(&f.settings()).0;
        let store = MusicStore::open(&f.settings(), &saved).unwrap();
        assert_eq!(
            store.collections().unwrap().playlists.lists[0].tracks,
            [path]
        );
    }
}
