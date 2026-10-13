use greyhack_behaviour::{FileKind, ROOT_USER};
use greyhack_world::{ComputerId, ComputerOptions, NetworkOptions, PlayerOptions, World};

/// A random file of a home: the path, its kind, owner, group and permissions.
pub type Entry = (String, FileKind, String, String, String);

pub async fn random_files(world: &World, computer: ComputerId) -> Vec<Entry> {
    let mut found = Vec::new();

    for user in world.users(computer).await.unwrap() {
        if user.name == ROOT_USER {
            continue;
        }

        let Some(home) = world
            .folder(computer, &format!("/home/{}", user.name), false)
            .await
            .unwrap()
        else {
            continue;
        };

        for file in world.files(&home).await.unwrap() {
            let Some(data) = file.data() else {
                continue;
            };

            if matches!(
                data.kind,
                FileKind::ChatLog | FileKind::Pdf | FileKind::Image
            ) {
                found.push((
                    file.path.clone(),
                    data.kind,
                    file.owner.clone(),
                    file.group.clone(),
                    file.permissions.as_str().to_owned(),
                ));
            }
        }
    }

    found.sort_by(|a, b| a.0.cmp(&b.0));

    found
}

pub async fn generated(computers: u32) -> (World, Vec<ComputerId>) {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let mut found = Vec::new();

    for _ in 0..computers {
        let router = world
            .add_network(&NetworkOptions {
                computers: Some(0),
                ..NetworkOptions::default()
            })
            .await
            .unwrap();

        found.push(
            world
                .add_computer_to(router, &ComputerOptions::default())
                .await
                .unwrap()
                .unwrap(),
        );
    }

    (world, found)
}
