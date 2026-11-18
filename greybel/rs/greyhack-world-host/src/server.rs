//! Accepts loopback connections to one world, counts them for the idle timeout, and resets the world.

use std::cell::{Cell, RefCell};
use std::fs::File;
use std::future::pending;
use std::io::{self, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use duplex_core::{Channel, ChannelOptions, ErrorCode, NextMessage, PendingConnection, Responder};
use duplex_driver_stdio::StreamDriver;
use greyhack_world::{DatabaseOpen, PlayerOptions, World};
use greyhack_world_core::{Database, WorldError};
use greyhack_world_operations::WorldServer;
use greyhack_world_programs::protocol::GameServer;
use scheduler::{Clock, TokioScheduler};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::sync::Notify;

use crate::options::{Lifetime, ServeOptions};
use crate::protocol::{self, Host, HostInfo, HostRouter, Reset, ResetFailure};
use crate::store::WorldStore;

#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    #[error("{0}")]
    Open(#[from] OpenError),
    #[error("port {port} can't be used: {source}")]
    Bind {
        port: u16,
        #[source]
        source: io::Error,
    },
    #[error("{0}")]
    LocalAddress(#[source] io::Error),
    #[error("{0}")]
    Output(#[source] io::Error),
    #[error("{0}")]
    Save(#[source] WorldError),
}

#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error("{0}")]
    World(#[from] WorldError),
    #[error("{} can't be opened: {source}", path.display())]
    Database {
        path: PathBuf,
        #[source]
        source: WorldError,
    },
}

/// World and host channels for one connection.
struct ConnectionChannel {
    world: Channel<GameServer, StreamDriver>,
    admin: Channel<Host, StreamDriver>,
}

pub struct WorldHost {
    options: ServeOptions,
    world: RefCell<WorldServer>,
    /// Folder the world is saved in, with the lock of its saved world.
    store: Option<(WorldStore, File)>,
    /// Channels of the connections. A reset closes their world channels.
    channels: RefCell<Vec<ConnectionChannel>>,
    /// Connections whose transport is open.
    connections: Cell<usize>,
    /// Wakes the idle timer when a connection opens or ends.
    activity: Notify,
}

impl WorldHost {
    /// Listens on `options.port` of the loopback address, opens the world, prints the port on the first output line,
    /// and serves until the server's lifetime ends. Saves a database world before returning.
    ///
    /// The port is bound before the world opens, so of two servers started at the same time on the same port, one
    /// fails fast.
    ///
    /// # Errors
    ///
    /// Returns failures to bind the port, open or save the world, or print the port.
    pub async fn serve(options: ServeOptions) -> Result<(), ServeError> {
        let listener = TcpListener::bind(("127.0.0.1", options.port))
            .await
            .map_err(|source| ServeError::Bind {
                port: options.port,
                source,
            })?;
        let port = listener
            .local_addr()
            .map_err(ServeError::LocalAddress)?
            .port();
        let (world, store) = Self::open(&options).await?;
        let host = Rc::new(Self {
            world: RefCell::new(world),
            store,
            options,
            channels: RefCell::default(),
            connections: Cell::new(0),
            activity: Notify::new(),
        });
        let mut stdout = io::stdout();

        writeln!(stdout, "{port}")
            .and_then(|()| stdout.flush())
            .map_err(ServeError::Output)?;

        let accepts = async {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    continue;
                };

                if let Ok(transport) = StreamDriver::tcp(stream) {
                    host.attach(&transport);
                }
            }
        };

        tokio::select! {
            () = accepts => {}
            () = host.end() => {}
        }

        let world = host.world.borrow().clone();

        world.save().await.map_err(ServeError::Save)
    }

    /// Attaches the world and host channels to `transport`. The connection ends after both channels close; if a peer
    /// opened only one, this happens when its transport disconnects.
    fn attach(self: &Rc<Self>, transport: &duplex_core::Transport<StreamDriver>) {
        let closes = Rc::new(Cell::new(0));
        let end = {
            let host = Rc::downgrade(self);

            move |_: &duplex_core::ConnectionEnd| {
                closes.set(closes.get() + 1);
                if closes.get() == 2
                    && let Some(host) = host.upgrade()
                {
                    host.connections.set(host.connections.get() - 1);
                    host.activity.notify_waiters();
                }
            }
        };
        let world = self.world.borrow().attach(transport);
        let admin = Channel::<Host, _>::new(transport, ChannelOptions::default());

        self.connections.set(self.connections.get() + 1);
        self.activity.notify_waiters();
        world.on_close(end.clone());
        admin.on_close(end);
        HostHandler(Rc::downgrade(self)).attach(&admin);
        self.channels.borrow_mut().retain(|channel| {
            channel.world.close_reason().is_none() || channel.admin.close_reason().is_none()
        });
        self.channels
            .borrow_mut()
            .push(ConnectionChannel { world, admin });
    }

    async fn end(&self) {
        match self.options.lifetime() {
            Lifetime::Input => {
                let mut input = tokio::io::stdin();
                let mut buffer = [0; 64];

                while input.read(&mut buffer).await.is_ok_and(|count| count > 0) {}
            }
            Lifetime::Idle(timeout) => self.idle(timeout).await,
            Lifetime::Endless => pending().await,
        }
    }

    async fn idle(&self, timeout: Duration) {
        loop {
            let activity = self.activity.notified();
            let mut activity = std::pin::pin!(activity);

            activity.as_mut().enable();
            if self.connections.get() > 0 {
                activity.await;
                continue;
            }

            tokio::select! {
                () = tokio::time::sleep(timeout) => return,
                () = activity => {}
            }
        }
    }

    /// Regenerates the world from the seed, replaces its saved world, and closes all world channels to interrupt
    /// programs. Does not reset a `--database` world.
    async fn reset(&self) -> Result<HostInfo, ResetFailure> {
        if self.options.database.is_some() {
            return Err(ResetFailure::Persistent);
        }

        let seed = &self.options.seed;
        let world = match &self.store {
            Some((store, _)) => store
                .regenerate(seed, Self::clock())
                .await
                .map_err(|error| error.to_string()),
            None => Self::generate(&self.options)
                .await
                .map_err(|error| error.to_string()),
        }
        .map_err(|message| ResetFailure::Generation { message })?;
        let previous = std::mem::replace(&mut *self.world.borrow_mut(), world);

        let worlds: Vec<_> = self
            .channels
            .borrow()
            .iter()
            .map(|channel| channel.world.clone())
            .collect();

        for world in worlds {
            world.close(ErrorCode::ClosedChannel);
        }

        drop(previous);
        if let Some((store, _)) = &self.store
            && let Some(file) = store.file_of(seed)
        {
            store.remove_stale(seed, &file);
        }

        Ok(HostInfo {
            seed: self.options.seed.clone(),
        })
    }

    async fn generate(options: &ServeOptions) -> Result<WorldServer, WorldError> {
        let world = World::from_seed(&options.seed, &PlayerOptions::default()).await?;

        WorldServer::new(world, Self::clock()).await
    }

    /// Opens the world: the seed's saved world in the worlds folder, a `--database` file, or with `--memory` a
    /// memory-only world. With a worlds folder that can't be used, the world is in memory only.
    async fn open(
        options: &ServeOptions,
    ) -> Result<(WorldServer, Option<(WorldStore, File)>), OpenError> {
        if options.memory {
            return Ok((Self::generate(options).await?, None));
        }

        if let Some(path) = &options.database {
            return Ok((Self::open_database(path, options).await?, None));
        }

        let Some(folder) = options.worlds.clone().or_else(WorldStore::default_folder) else {
            eprintln!("no data folder is known, so the world stays in memory");

            return Ok((Self::generate(options).await?, None));
        };
        let store = WorldStore::new(folder);

        match store.open(&options.seed, Self::clock()).await {
            Ok(world) => Ok((world.server, Some((store, world.lock)))),
            Err(error) => {
                eprintln!("{error}, so the world stays in memory");

                Ok((Self::generate(options).await?, None))
            }
        }
    }

    /// Loads the world of the database at `path`, or generates it there when the file doesn't exist.
    async fn open_database(
        path: &std::path::Path,
        options: &ServeOptions,
    ) -> Result<WorldServer, OpenError> {
        let exists = path.exists();
        let database = Database::open(&path.to_string_lossy())
            .await
            .map_err(|error| OpenError::Database {
                path: path.to_owned(),
                source: WorldError::from(error),
            })?;

        if exists {
            return Ok(WorldServer::open(database, Self::clock()).await?);
        }

        let world = World::from_seed(&options.seed, &PlayerOptions::default()).await?;

        Ok(WorldServer::with_database(world, database, Self::clock()).await?)
    }

    fn clock() -> Rc<dyn Clock> {
        Rc::new(TokioScheduler::new())
    }
}

/// Handles host messages for one connection.
struct HostHandler(std::rc::Weak<WorldHost>);

impl HostRouter for HostHandler {
    fn on_connect(&self, (): (), connection: PendingConnection<protocol::WorldHost>) {
        if let Some(host) = self.0.upgrade() {
            connection.accept(HostInfo {
                seed: host.options.seed.clone(),
            });
        }
    }

    fn on_reset(
        &self,
        _message: Reset,
        responder: Responder<HostInfo, ResetFailure>,
        _next: NextMessage<Reset>,
    ) {
        let Some(host) = self.0.upgrade() else {
            return;
        };

        tokio::task::spawn_local(async move {
            match host.reset().await {
                Ok(info) => responder.respond(info),
                Err(failure) => responder.fail(failure),
            }
        });
    }
}
