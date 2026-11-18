//! Folder of saved worlds, one database per seed, listed in its `worlds.json`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use greyhack_world::{DatabaseOpen, PlayerOptions, World};
use greyhack_world_core::{Database, WorldError};
use greyhack_world_operations::WorldServer;
use scheduler::Clock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const INDEX: &str = "worlds.json";

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{}: {source}", path.display())]
    File {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("another world server uses the saved world of seed \"{seed}\"")]
    Busy { seed: String },
    #[error("the saved world of seed \"{seed}\" is from a newer Greybel version")]
    Newer { seed: String },
    #[error("{0}")]
    World(#[from] WorldError),
}

/// Saved worlds by seed.
#[derive(Default, Serialize, Deserialize)]
struct Index {
    worlds: BTreeMap<String, Entry>,
}

/// Saved world of one seed.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    /// Database file in the folder.
    file: String,
    /// Unix time in seconds the world was generated.
    created: u64,
    /// Unix time in seconds a server last opened the world.
    last_used: u64,
    /// Database layout the world was saved with; absent in worlds saved before layouts were recorded.
    #[serde(default)]
    format: Option<Format>,
}

/// Database layout of a world: the schema version of each world domain and a hash of all table statements. The hash
/// changes when a table changes without a version step.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Format {
    versions: BTreeMap<String, u32>,
    schema: String,
}

/// How a saved world's layout relates to this build's.
enum Compatibility {
    Same,
    /// No layout recorded, or older domain versions that the database migrates when it opens.
    Migratable,
    /// Same domain versions with different tables, which the database can't migrate.
    TableMismatch,
    /// A domain version newer than this build supports.
    Newer,
}

impl Format {
    /// Layout of this build's world schema.
    fn current() -> Self {
        let schema = greyhack_world::world_schema();
        let hash = Sha256::digest(schema.sql().as_bytes());

        Self {
            versions: schema
                .domains()
                .iter()
                .map(|domain| (domain.name.to_owned(), domain.version))
                .collect(),
            schema: hash[..8].iter().fold(String::new(), |mut text, byte| {
                let _ = write!(text, "{byte:02x}");

                text
            }),
        }
    }

    /// Compares a saved world's layout with this build's.
    fn compare(&self, entry_format: Option<&Self>) -> Compatibility {
        let Some(entry_format) = entry_format else {
            return Compatibility::Migratable;
        };

        if entry_format == self {
            return Compatibility::Same;
        }

        let newer = entry_format.versions.iter().any(|(domain, version)| {
            self.versions
                .get(domain)
                .is_none_or(|current| version > current)
        });

        if newer {
            Compatibility::Newer
        } else if entry_format.versions == self.versions {
            Compatibility::TableMismatch
        } else {
            Compatibility::Migratable
        }
    }
}

pub(crate) struct WorldStore {
    folder: PathBuf,
}

/// Saved world a server opened. Other servers can't open it while this one has its lock.
pub(crate) struct StoredWorld {
    pub server: WorldServer,
    pub lock: File,
}

impl WorldStore {
    pub(crate) fn new(folder: PathBuf) -> Self {
        Self { folder }
    }

    /// `greybel/worlds` in the user's data folder: Application Support on macOS, `%APPDATA%` on Windows, and
    /// `$XDG_DATA_HOME` or `~/.local/share` elsewhere.
    pub(crate) fn default_folder() -> Option<PathBuf> {
        let home = || std::env::var_os("HOME").map(PathBuf::from);
        let data = if cfg!(target_os = "macos") {
            home().map(|home| home.join("Library/Application Support"))
        } else if cfg!(windows) {
            std::env::var_os("APPDATA").map(PathBuf::from)
        } else {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| home().map(|home| home.join(".local/share")))
        };

        data.map(|data| data.join("greybel").join("worlds"))
    }

    /// Opens the saved world of `seed`, generating and saving it when the folder has none. A world whose layout this
    /// build can't migrate, or that fails to load, is renamed with an `.outdated` or `.unreadable` suffix and
    /// generated again. A newer-layout world is not modified.
    pub(crate) async fn open(
        &self,
        seed: &str,
        clock: Rc<dyn Clock>,
    ) -> Result<StoredWorld, StoreError> {
        self.create_folder()?;

        let lock = self.lock(seed)?;
        let current = Format::current();
        let entry = self.change_index(|index| {
            let entry = index.worlds.get_mut(seed)?;

            entry.last_used = Self::now();

            Some(entry.clone())
        })?;

        if let Some(entry) = entry {
            let path = self.folder.join(&entry.file);

            self.remove_stale(seed, &entry.file);

            if path.exists() {
                match current.compare(entry.format.as_ref()) {
                    Compatibility::Newer => {
                        return Err(StoreError::Newer {
                            seed: seed.to_owned(),
                        });
                    }
                    Compatibility::TableMismatch => {
                        Self::set_aside(
                            &path,
                            "outdated",
                            "this Greybel version stores worlds differently",
                        )?;
                    }
                    Compatibility::Same | Compatibility::Migratable => {
                        match Self::load(&path, Rc::clone(&clock)).await {
                            Ok(server) => {
                                if entry.format.as_ref() != Some(&current) {
                                    self.record(seed, &entry.file, entry.created, &current)?;
                                }

                                return Ok(StoredWorld { server, lock });
                            }
                            Err(error) => Self::set_aside(&path, "unreadable", &error.to_string())?,
                        }
                    }
                }
            }
        }

        let server = self.regenerate(seed, clock).await?;

        Ok(StoredWorld { server, lock })
    }

    /// Generates the world of `seed` again into a new file, which replaces the seed's saved world. Call with the
    /// seed's lock; the old file is removed once no server has it open.
    pub(crate) async fn regenerate(
        &self,
        seed: &str,
        clock: Rc<dyn Clock>,
    ) -> Result<WorldServer, StoreError> {
        let file = Self::file_name(seed);
        let server = Self::generate(seed, &self.folder.join(&file), clock).await?;

        self.record(seed, &file, Self::now(), &Format::current())?;

        Ok(server)
    }

    /// Records `file` as `seed`'s saved world, with its creation time and layout.
    fn record(
        &self,
        seed: &str,
        file: &str,
        creation: u64,
        format: &Format,
    ) -> Result<(), StoreError> {
        self.change_index(|index| {
            index.worlds.insert(
                seed.to_owned(),
                Entry {
                    file: file.to_owned(),
                    created: creation,
                    last_used: Self::now(),
                    format: Some(format.clone()),
                },
            );
        })
    }

    /// Renames the database at `path` and its write-ahead log with `suffix`, reporting `reason`.
    fn set_aside(path: &Path, suffix: &str, reason: &str) -> Result<(), StoreError> {
        let mut target = path.as_os_str().to_owned();

        target.push(format!(".{suffix}"));
        eprintln!(
            "the saved world {} is stored as {} and generated again: {reason}",
            path.display(),
            Path::new(&target).display()
        );

        for extension in ["", "-wal", "-shm"] {
            let mut from = path.as_os_str().to_owned();
            let mut to = from.clone();

            from.push(extension);
            to.push(extension);
            to.push(format!(".{suffix}"));

            if Path::new(&from).exists() {
                fs::rename(&from, &to).map_err(|source| StoreError::File {
                    path: from.into(),
                    source,
                })?;
            }
        }

        Ok(())
    }

    /// Removes stale database files for `seed`, excluding `file` and files set aside as unreadable or outdated.
    pub(crate) fn remove_stale(&self, seed: &str, file: &str) {
        let prefix = format!("{}.", Self::key(seed));
        let Ok(entries) = fs::read_dir(&self.folder) else {
            return;
        };

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();

            if name.starts_with(&prefix)
                && name.contains(".db")
                && !name.ends_with(".unreadable")
                && !name.ends_with(".outdated")
                && !name.starts_with(file)
            {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    pub(crate) fn file_of(&self, seed: &str) -> Option<String> {
        self.read_index()
            .worlds
            .get(seed)
            .map(|entry| entry.file.clone())
    }

    async fn load(path: &Path, clock: Rc<dyn Clock>) -> Result<WorldServer, WorldError> {
        let database = Database::open(&path.to_string_lossy()).await?;

        WorldServer::open(database, clock).await
    }

    async fn generate(
        seed: &str,
        path: &Path,
        clock: Rc<dyn Clock>,
    ) -> Result<WorldServer, WorldError> {
        let world = World::from_seed(seed, &PlayerOptions::default()).await?;
        let database = Database::open(&path.to_string_lossy()).await?;

        WorldServer::with_database(world, database, clock).await
    }

    fn create_folder(&self) -> Result<(), StoreError> {
        fs::create_dir_all(&self.folder).map_err(|source| StoreError::File {
            path: self.folder.clone(),
            source,
        })
    }

    /// Locks the saved world of `seed` for this process.
    fn lock(&self, seed: &str) -> Result<File, StoreError> {
        let path = self.folder.join(format!("{}.lock", Self::key(seed)));
        let file = Self::open_lock(&path)?;

        match file.try_lock() {
            Ok(()) => Ok(file),
            Err(fs::TryLockError::WouldBlock) => Err(StoreError::Busy {
                seed: seed.to_owned(),
            }),
            Err(fs::TryLockError::Error(source)) => Err(StoreError::File { path, source }),
        }
    }

    /// Reads the index, applies `change` and writes it, with the index lock against other servers.
    fn change_index<T>(&self, change: impl FnOnce(&mut Index) -> T) -> Result<T, StoreError> {
        let lock_path = self.folder.join(format!("{INDEX}.lock"));
        let lock = Self::open_lock(&lock_path)?;

        lock.lock().map_err(|source| StoreError::File {
            path: lock_path,
            source,
        })?;

        let mut index = self.read_index();
        let value = change(&mut index);
        let path = self.folder.join(INDEX);
        let draft = self.folder.join(format!("{INDEX}.draft"));
        let text = serde_json::to_string_pretty(&index).expect("the index serializes");

        fs::write(&draft, text + "\n")
            .and_then(|()| fs::rename(&draft, &path))
            .map_err(|source| StoreError::File { path, source })?;

        Ok(value)
    }

    /// Reads the index; an unreadable index is stored with an `.unreadable` suffix and read as empty.
    fn read_index(&self) -> Index {
        let path = self.folder.join(INDEX);
        let Ok(text) = fs::read_to_string(&path) else {
            return Index::default();
        };

        serde_json::from_str(&text).unwrap_or_else(|error| {
            eprintln!(
                "{} can't be read and is stored with an .unreadable suffix: {error}",
                path.display()
            );

            let _ = fs::rename(&path, path.with_extension("json.unreadable"));

            Index::default()
        })
    }

    fn open_lock(path: &Path) -> Result<File, StoreError> {
        OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)
            .map_err(|source| StoreError::File {
                path: path.to_owned(),
                source,
            })
    }

    fn file_name(seed: &str) -> String {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |time| time.as_millis());

        format!("{}.{millis}.db", Self::key(seed))
    }

    /// Readable, unique file name part for `seed`: its letters, digits, `-` and `_`, then a hash of the entire seed.
    fn key(seed: &str) -> String {
        let readable: String = seed
            .chars()
            .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
            .take(32)
            .collect();
        let hash = Sha256::digest(seed.as_bytes());

        format!(
            "{readable}-{:02x}{:02x}{:02x}{:02x}",
            hash[0], hash[1], hash[2], hash[3]
        )
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |time| time.as_secs())
    }
}
