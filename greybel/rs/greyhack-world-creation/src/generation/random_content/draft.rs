use greyhack_behaviour::{PermissionSet, path};
use greyhack_world_libraries::{ComputerDraft, NodeDraft};
use mono_random::Random;

use super::RandomContent;

/// Adds the random files and images of `computer` (`Router.GenerateRandomContent`) drawn from
/// `seed`, which the game derives from the network seed and the computer's LAN address. Files
/// whose name a home already has are skipped, images replace entries of their name.
pub(crate) fn add_to_draft(computer: &mut ComputerDraft, seed: i32, neurobox: bool) {
    let users: Vec<String> = computer
        .users
        .iter()
        .map(|user| user.name.clone())
        .collect();
    let fs = &computer.filesystem;
    let content = RandomContent::draw(
        &mut Random::new(seed),
        &users,
        neurobox,
        &|user, name| {
            fs.folder(&path::home(user), false)
                .is_some_and(|home| fs.has_child(home, name))
        },
        &|_| false,
    );

    for file in content.files {
        let fs = &mut computer.filesystem;
        let Some(home) = fs.folder(&path::home(&file.user), false) else {
            continue;
        };

        fs.add(
            home,
            NodeDraft::file(
                &file.name,
                &file.user,
                PermissionSet::default_for(false),
                file.data,
            ),
        );
    }
}
