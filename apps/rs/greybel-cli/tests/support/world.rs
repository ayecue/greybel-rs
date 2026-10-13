//! Test game server on TCP, allowing separate `greybel` processes to share one generated world.

use std::rc::Rc;
use std::sync::mpsc;
use std::thread;

use duplex_driver_stdio::StreamDriver;
use greyhack_world::{PlayerOptions, World};
use greyhack_world_operations::WorldServer;
use scheduler::TokioScheduler;
use tokio::net::TcpListener;
use tokio::task::LocalSet;

/// Starts a dedicated thread serving a world generated from seed `test` and returns its TCP address. The server runs
/// for the remainder of the test process.
pub fn serve() -> String {
    let (sender, address) = mpsc::channel();

    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Tokio's runtime builds");

        LocalSet::new().block_on(&runtime, async move {
            let world = World::from_seed("test", &PlayerOptions::default())
                .await
                .expect("the world generates");
            let server = WorldServer::new(world, Rc::new(TokioScheduler::new()));
            let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("binds");
            let port = listener.local_addr().expect("an address").port();

            sender.send(format!("127.0.0.1:{port}")).expect("sends");

            loop {
                let (stream, _) = listener.accept().await.expect("accepts");

                server.attach(&StreamDriver::tcp(stream).expect("the connection is set up"));
            }
        });
    });

    address.recv().expect("the game server listens")
}
