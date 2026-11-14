use greyhack_world::{
    Coin, Computer, Hardware, Library, Node, Port, Router, ScheduledRoutine, User, World,
};

use crate::support::{computers, routers};

#[derive(Debug, PartialEq)]
pub struct ComputerSnapshot {
    row: Computer,
    users: Vec<User>,
    ports: Vec<Port>,
    hardware: Hardware,
    nodes: Vec<Node>,
    libraries: Vec<Option<Library>>,
}

#[derive(Debug, PartialEq)]
pub struct WorldSnapshot {
    routers: Vec<(Router, Option<String>)>,
    computers: Vec<ComputerSnapshot>,
    coins: Vec<Coin>,
    routines: Vec<ScheduledRoutine>,
    date: String,
}

pub async fn snapshot(world: &World) -> WorldSnapshot {
    let mut router_rows = Vec::new();

    for router in routers(world).await {
        let kernel = world
            .modules
            .core
            .networks
            .routers
            .kernel_version(router.id)
            .await
            .unwrap();

        router_rows.push((router, kernel));
    }

    let mut computer_rows = Vec::new();

    for computer in computers(world).await {
        let root = world
            .modules
            .core
            .files
            .lookup
            .root(computer.id)
            .await
            .unwrap()
            .expect("a root");
        let mut stack = vec![root];
        let mut nodes = Vec::new();
        let mut libraries = Vec::new();

        while let Some(node) = stack.pop() {
            let children = world
                .modules
                .core
                .files
                .lookup
                .children(&node)
                .await
                .unwrap();

            stack.extend(children.into_iter().rev());
            libraries.push(
                world
                    .modules
                    .libraries
                    .generator
                    .full_library_of(&node)
                    .await
                    .unwrap(),
            );
            nodes.push(node);
        }

        computer_rows.push(ComputerSnapshot {
            users: world
                .modules
                .core
                .computers
                .users
                .users(computer.id)
                .await
                .unwrap(),
            ports: world
                .modules
                .core
                .computers
                .processes
                .ports(computer.id)
                .await
                .unwrap(),
            hardware: world
                .modules
                .core
                .computers
                .hardware
                .hardware(computer.id)
                .await
                .unwrap(),
            row: computer,
            nodes,
            libraries,
        });
    }

    WorldSnapshot {
        routers: router_rows,
        computers: computer_rows,
        coins: world.modules.economy.coins.coins().await.unwrap(),
        routines: world.modules.routines.scheduled_routines().await.unwrap(),
        date: world.date(),
    }
}
