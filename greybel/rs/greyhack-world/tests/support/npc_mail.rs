use greyhack_world::{
    ComputerOptions, Email, JobRole, NetworkOptions, PersonId, PlayerOptions, WorkPerformance,
    World,
};

pub const PASSWORD: &str = "secret";
const NPC_COUNT: usize = 12;

pub struct Member {
    pub person: PersonId,
    pub address: String,
}

/// A network of `NPC_COUNT` NPC workstations whose mail passwords are `PASSWORD`.
pub async fn network() -> (World, String, Vec<Member>) {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let router = world
        .add_network(&NetworkOptions {
            computers: Some(0),
            domain: Some("corp.com".to_owned()),
            ..NetworkOptions::default()
        })
        .await
        .unwrap();
    let public_ip = world.router(router).await.unwrap().unwrap().public_ip;
    let now = world.time_seconds();
    let mut members = Vec::new();

    for index in 0..NPC_COUNT {
        let user = format!("worker{index}");
        let computer = world
            .add_computer_to(
                router,
                &ComputerOptions {
                    users: Some(vec![(user.clone(), "pw".to_owned())]),
                    services: Some(Vec::new()),
                    ..ComputerOptions::default()
                },
            )
            .await
            .unwrap()
            .unwrap();
        let person = world
            .add_npc(computer, &user, &format!("First{index} {user}"), now)
            .await
            .unwrap()
            .unwrap();
        let performance = match index % 3 {
            0 => WorkPerformance::VeryPoor,
            1 => WorkPerformance::Normal,
            _ => WorkPerformance::Excellent,
        };

        world
            .set_npc_work_performance(person, performance)
            .await
            .unwrap();
        world
            .set_npc_job(person, JobRole::Sales, None)
            .await
            .unwrap();

        let address = world
            .person(person)
            .await
            .unwrap()
            .unwrap()
            .mail_address
            .unwrap();

        world
            .restore_mail_account(&address, PASSWORD, false, &[])
            .await
            .unwrap();
        members.push(Member { person, address });
    }

    (world, public_ip, members)
}

pub async fn inbox(world: &World, address: &str) -> Vec<Email> {
    world.mail_account(address).await.unwrap().unwrap().inbox
}

pub async fn inboxes(world: &World, members: &[Member]) -> Vec<Vec<Email>> {
    let mut all = Vec::new();

    for member in members {
        all.push(inbox(world, &member.address).await);
    }

    all
}
