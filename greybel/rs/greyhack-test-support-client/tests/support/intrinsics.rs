use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;

use greybel_interpreter::{InputFuture, InterpreterOptions, Key, Stdin, Stdout};
use greybel_test_support::future::block_on;
use greybel_test_support::greyhack::{
    run_in, run_on_host_at, run_on_host_at_async, run_on_host_at_with_loaded_options,
};
use greybel_test_support::interpreter::{constant_clock, constant_clock_options};
use greybel_test_support::io::MarkedStdout;
use greyhack_behaviour::{AlarmConfig, CompanyAlign, JobRole, NpcSchedule, WorkPerformance};
use greyhack_test_support_client::protocol::test_support::{MailSnapshot, NetworkSnapshot};
use greyhack_world::{DatabaseOpen, PlayerOptions, World};
use greyhack_world_core::Database;
use greyhack_world_libraries::{NpcDebugCredentials, ZeroDayRumor};
use greyhack_world_npcs::{Email, MailSpamConfig};
use greyhack_world_operations::WorldServer;
use greyhack_world_player::CtfEvent;
use greyhack_world_smart_devices::TrafficNet;
use scheduler::ManualClock;

use super::mirror::economy;

/// The name of the program the comparisons run, in the game and the intrinsics.
pub const PROGRAM_NAME: &str = "test";

/// Runs `scripts` in order in one world, matching the shared state of successive game executions.
pub fn run_many_with_world(
    scripts: &[&str],
    user: &str,
    world: World,
    mut options: impl FnMut() -> InterpreterOptions,
) -> Vec<(String, Option<String>)> {
    let mut world = Some(world);
    let path = format!("/home/{user}/{PROGRAM_NAME}");

    scripts
        .iter()
        .map(|script| {
            let run = run_on_host_at(
                script,
                world.take().expect("the previous run returned its world"),
                user,
                true,
                &path,
                options(),
            );

            world = Some(run.world);

            (run.terminal, run.error)
        })
        .collect()
}

/// Runs `scripts` in order in one simulated world, matching the shared state of successive game executions.
pub fn run_many_in_snapshot(
    scripts: &[&str],
    user: &str,
    snapshot: &NetworkSnapshot,
) -> Vec<(String, Option<String>)> {
    run_many_with_world(scripts, user, snapshot_world(user, snapshot), || {
        constant_clock_options()
    })
}

/// Runs `scripts` in order in one new-player world.
pub fn run_many_on_new_player(scripts: &[&str], user: &str) -> Vec<(String, Option<String>)> {
    let player = PlayerOptions {
        user: user.to_owned(),
        mail_address: None,
        bank_number: None,
        libraries: Vec::new(),
        ..PlayerOptions::default()
    };
    let world = block_on(World::new(&player)).expect("a world");

    run_many_with_world(scripts, user, world, constant_clock_options)
}

/// [`run_many_in_snapshot`] with a fresh wait-advancing clock for each program.
pub fn run_many_in_snapshot_with_wait_clock(
    scripts: &[&str],
    user: &str,
    snapshot: &NetworkSnapshot,
) -> Vec<(String, Option<String>)> {
    run_many_with_world(scripts, user, snapshot_world(user, snapshot), || {
        InterpreterOptions::new(Rc::new(ManualClock::advance_on_sleep()))
    })
}

/// [`run_many_in_snapshot`] with real waits and world ticks for each program.
pub fn run_many_in_snapshot_real_time(
    scripts: &[&str],
    user: &str,
    snapshot: &NetworkSnapshot,
) -> Vec<(String, Option<String>)> {
    run_many_with_world(scripts, user, snapshot_world(user, snapshot), || {
        InterpreterOptions::new(Rc::new(scheduler::TokioScheduler::new()))
    })
}

/// Runs `scripts` on the base snapshot with additional networks available without joining them.
pub fn run_many_in_snapshots(
    scripts: &[&str],
    user: &str,
    base: &NetworkSnapshot,
    additional: &[NetworkSnapshot],
) -> Vec<(String, Option<String>)> {
    let world = snapshot_world(user, base);

    for network in additional {
        block_on(
            world
                .modules
                .creation
                .network_generation
                .add_network_layout(&super::mirror::layout(network)),
        )
        .expect("the additional network is mirrored");
    }

    run_many_with_world(scripts, user, world, constant_clock_options)
}

/// Runs `scripts` after priming each computer's flood count with distinct fake public origins.
pub fn run_many_in_snapshot_with_floods(
    scripts: &[&str],
    user: &str,
    snapshot: &NetworkSnapshot,
    floods: &[(String, String, u32)],
) -> Vec<(String, Option<String>)> {
    let world = snapshot_world(user, snapshot);

    for (address, origin, threshold) in floods {
        let computer = block_on(
            world
                .modules
                .core
                .networks
                .routers
                .computer_at(&snapshot.public_ip, address),
        )
        .expect("the flooded computer is read")
        .expect("the flooded computer exists");

        block_on(
            world
                .modules
                .intrusion
                .floods
                .flood(computer, origin, *threshold),
        )
        .expect("the flood is primed");
    }

    run_many_with_world(scripts, user, world, constant_clock_options)
}

/// Async snapshot run for a `LocalSet`, without moving the non-`Send` world to another thread.
pub async fn run_in_snapshot_world_async(
    code: &str,
    user: &str,
    snapshot: &NetworkSnapshot,
) -> (String, Option<String>, World) {
    run_in_snapshot_world_with_async(code, user, snapshot, constant_clock_options()).await
}

/// Async interactive snapshot run for a `LocalSet`.
pub async fn run_interactive_in_snapshot_world_async(
    code: &str,
    user: &str,
    snapshot: &NetworkSnapshot,
    inputs: &[String],
) -> (String, Option<String>, World) {
    run_in_snapshot_world_with_async(
        code,
        user,
        snapshot,
        InterpreterOptions {
            stdin: Rc::new(InteractiveStdin::new(inputs)),
            ..InterpreterOptions::new(Rc::new(scheduler::TokioScheduler::new()))
        },
    )
    .await
}

async fn run_in_snapshot_world_with_async(
    code: &str,
    user: &str,
    snapshot: &NetworkSnapshot,
    options: InterpreterOptions,
) -> (String, Option<String>, World) {
    let world = snapshot_world(user, snapshot);
    let path = format!("/home/{user}/{PROGRAM_NAME}");
    let run = run_on_host_at_async(code, world, user, true, &path, options).await;

    (run.terminal, run.error, run.world)
}

fn run_in_snapshot_with_options(
    code: &str,
    user: &str,
    snapshot: &NetworkSnapshot,
    options: InterpreterOptions,
) -> (String, Option<String>) {
    let world = snapshot_world(user, snapshot);
    let path = format!("/home/{user}/{PROGRAM_NAME}");
    let run = run_on_host_at(code, world, user, true, &path, options);

    (run.terminal, run.error)
}

/// Queued line and key input, matching the game's `WaitInput` return value (the input itself, not its prompt).
struct InteractiveStdin {
    inputs: RefCell<VecDeque<String>>,
    poll_started: Cell<bool>,
    before_line: Option<Rc<dyn Fn()>>,
}

impl InteractiveStdin {
    fn new(inputs: &[String]) -> Self {
        Self {
            inputs: RefCell::new(inputs.iter().cloned().collect()),
            poll_started: Cell::new(false),
            before_line: None,
        }
    }

    fn with_action(inputs: &[String], before_line: Rc<dyn Fn()>) -> Self {
        Self {
            inputs: RefCell::new(inputs.iter().cloned().collect()),
            poll_started: Cell::new(false),
            before_line: Some(before_line),
        }
    }

    fn key(&self) -> Option<Key> {
        self.inputs
            .borrow_mut()
            .pop_front()
            .and_then(|input| input.chars().next())
            .map(Key::Char)
    }
}

impl Stdin for InteractiveStdin {
    fn read_line(&self, _prompt: &str, _echo: greybel_interpreter::Echo) -> InputFuture<String> {
        if let Some(action) = &self.before_line {
            action();
        }
        let line = self.inputs.borrow_mut().pop_front().unwrap_or_default();

        Box::pin(std::future::ready(Ok(line)))
    }

    fn read_key(&self, _prompt: &str) -> InputFuture<Key> {
        let key = self.key().unwrap_or(Key::Enter);

        Box::pin(std::future::ready(Ok(key)))
    }

    fn try_read_key(&self, _prompt: &str) -> InputFuture<Option<Key>> {
        if !self.poll_started.replace(true) {
            return Box::pin(std::future::ready(Ok(None)));
        }

        let key = self.key();

        Box::pin(std::future::ready(Ok(key)))
    }
}

/// Runs an interactive script with the same queued inputs the game fixture types after its marker.
pub fn run_interactive_in_snapshot(
    code: &str,
    user: &str,
    snapshot: &NetworkSnapshot,
    inputs: &[String],
) -> (String, Option<String>) {
    run_in_snapshot_with_options(
        code,
        user,
        snapshot,
        InterpreterOptions {
            stdin: Rc::new(InteractiveStdin::new(inputs)),
            ..InterpreterOptions::new(Rc::new(scheduler::TokioScheduler::new()))
        },
    )
}

/// Runs an interactive script and invokes `action` immediately before each line input.
pub fn run_interactive_action_in_snapshot(
    code: &str,
    user: &str,
    snapshot: &NetworkSnapshot,
    inputs: &[String],
    action: Arc<dyn Fn(&World) + Send + Sync>,
) -> (String, Option<String>) {
    let world = snapshot_world(user, snapshot);
    let path = format!("/home/{user}/{PROGRAM_NAME}");
    let inputs = inputs.to_vec();
    let run =
        run_on_host_at_with_loaded_options(code, world, user, true, &path, move |action_world| {
            InterpreterOptions {
                stdin: Rc::new(InteractiveStdin::with_action(
                    &inputs,
                    Rc::new(move || {
                        action(&action_world);
                        // The input read is a read cycle; persist fixture mutations before the next program operation.
                        let _ = block_on(action_world.database().conn.execute("COMMIT", ()));
                    }),
                )),
                ..InterpreterOptions::new(Rc::new(scheduler::TokioScheduler::new()))
            }
        });

    (run.terminal, run.error)
}

/// Builds the mirrored world once, so two programs can share it.
pub fn snapshot_world(user: &str, snapshot: &NetworkSnapshot) -> World {
    let bank = snapshot.player_bank.as_ref().map(|number| {
        let money = snapshot
            .banks
            .iter()
            .find(|bank| bank.number == *number)
            .map_or(0.0, |bank| bank.money);

        (number.clone(), money)
    });
    let player = PlayerOptions {
        user: user.to_owned(),
        mail_address: snapshot
            .player_mail
            .as_ref()
            .map(|mail| mail.address.clone()),
        mail_password: snapshot
            .player_mail
            .as_ref()
            .map(|mail| mail.password.clone()),
        bank_number: bank.as_ref().map(|(number, _)| number.clone()),
        bank_money: bank.as_ref().map_or(0.0, |(_, money)| *money),
        libraries: Vec::new(),
        date: snapshot.current_date.clone(),
        network: Some(super::mirror::layout(snapshot)),
        ..PlayerOptions::default()
    };
    let world = block_on(World::new(&player)).expect("a world");

    world.set_world_secret(snapshot.world_secret.to_vec());
    super::mirror::register_latest_libraries(&world, snapshot);

    if let Some(debug) = &snapshot.debug {
        block_on(
            world
                .modules
                .core
                .computers
                .users
                .register_password(&debug.password),
        )
        .expect("the debug password is registered");
        let rumor = ZeroDayRumor {
            npc_passwords: [(
                debug.user.clone(),
                NpcDebugCredentials {
                    password: debug.password.clone(),
                    player_id: None,
                },
            )]
            .into_iter()
            .collect(),
            ..ZeroDayRumor::default()
        };
        block_on(world.modules.libraries.debug.restore_rumor(&rumor))
            .expect("the mirrored zero-day credentials");
    }

    if let Some(ctf) = &snapshot.ctf {
        block_on(world.modules.player.ctf.create_ctf_account(
            world.handle.player(),
            &ctf.user,
            &ctf.password,
        ))
        .expect("the mirrored CTF account");
        block_on(
            world.modules.player.ctf.create_ctf_event(&CtfEvent {
                name: ctf.name.clone(),
                user: ctf.user.clone(),
                password_hash: greyhack_behaviour::hash_password(&ctf.password),
                description: ctf.description.clone(),
                public_address: ctf.public_address.clone(),
                creator_name: ctf.creator_name.clone(),
                mail_content: ctf.mail_content.clone(),
                creator: world.handle.player_person(),
                creator_bank_user: snapshot
                    .player_bank
                    .as_ref()
                    .map_or_else(String::new, ToString::to_string),
                quality_rank: 3,
                difficulty_rank: 3,
                num_votes: 1,
                is_rated: false,
            }),
        )
        .expect("the mirrored CTF event");
        block_on(
            world
                .modules
                .player
                .ctf
                .add_ctf_player(&ctf.name, world.handle.player()),
        )
        .expect("the mirrored CTF participant");
        if ctf.completed {
            block_on(
                world
                    .modules
                    .player
                    .ctf
                    .player_success(&ctf.name, world.handle.player()),
            )
            .expect("the mirrored CTF completion");
        }
    }

    let mut npc_ids = std::collections::HashMap::new();

    for npc in &snapshot.npcs {
        let computer = block_on(
            world
                .modules
                .core
                .networks
                .routers
                .computer_at(&snapshot.public_ip, &npc.local_ip),
        )
        .expect("the NPC workstation is read")
        .expect("the mirrored NPC workstation");
        let users = block_on(world.modules.core.computers.users.users(computer))
            .expect("the NPC workstation accounts are read");

        if !users.iter().any(|user| user.name == npc.user) {
            continue;
        }

        let person = block_on(world.modules.npcs.npcs.add_npc(
            computer,
            &npc.user,
            &npc.name,
            world.handle.time_seconds(),
        ))
        .expect("the mirrored NPC is added")
        .expect("the mirrored NPC account");

        block_on(world.modules.npcs.jobs.set_npc_job(
            person,
            JobRole::from_code(npc.job_role).expect("the game's job role"),
            (!npc.license_plate.is_empty()).then(|| npc.license_plate.clone()),
        ))
        .expect("the mirrored NPC job");
        block_on(world.modules.npcs.schedules.set_npc_schedule(
            person,
            NpcSchedule::from_schedule(
                npc.schedule_work1_start,
                npc.schedule_work2_start,
                npc.walk_to_appliance,
            ),
        ))
        .expect("the mirrored NPC schedule");
        block_on(world.modules.npcs.schedules.set_npc_alarm_state(
            person,
            AlarmConfig {
                hour: npc.alarm_hour,
                minute: npc.alarm_minute,
                is_modified: npc.alarm_modified,
                is_deleted: npc.alarm_deleted,
            },
            npc.wake_up_time,
            npc.day_alarm,
            npc.late_strikes,
            npc.fired,
        ))
        .expect("the mirrored NPC alarm");
        // Logged in as in the game: otherwise the first switch would log them in again, which for the network's
        // administrator runs the network restore the game ran when they came in (`Persona.NextCita`).
        block_on(
            world
                .modules
                .npcs
                .schedules
                .set_npc_login(person, npc.logged),
        )
        .expect("the mirrored NPC login");

        let forced_until = npc
            .forced_login_seconds
            .map(|seconds| world.handle.time_seconds() + seconds);

        block_on(
            world
                .modules
                .npcs
                .schedules
                .set_npc_forced_login(person, forced_until),
        )
        .expect("the mirrored forced login");
        block_on(world.modules.npcs.jobs.set_npc_work_performance(
            person,
            WorkPerformance::from_code(npc.work_performance).expect("the game's work performance"),
        ))
        .expect("the mirrored work performance");
        block_on(world.modules.npcs.jobs.set_npc_company_align(
            person,
            CompanyAlign::from_code(npc.company_align).expect("the game's company alignment"),
            (!npc.day_zero_contact.is_empty()).then(|| npc.day_zero_contact.clone()),
            (npc.company_align != 0).then_some(0),
        ))
        .expect("the mirrored company alignment");
        if let Some(mail) = &npc.mail {
            // The game's address is `surname@domain` except for a mission NPC that is already on another computer of
            // the network, which keeps the address of the NPC it replaced (`NpcInfo.UpdateNpcInfo`'s `mailUserAlias`).
            block_on(
                world
                    .modules
                    .npcs
                    .npcs
                    .set_person_mail_address(person, &mail.address),
            )
            .expect("the mirrored NPC mail address");
            block_on(world.modules.npcs.mail.restore_mail_account(
                &mail.address,
                &mail.password,
                mail.generated,
                &mail_emails(&mail.emails),
            ))
            .expect("the mirrored NPC mailbox");
        }

        if snapshot
            .admin
            .as_ref()
            .is_some_and(|admin| admin.persona_exists && admin.user == npc.user)
        {
            block_on(world.modules.npcs.npcs.appoint_network_admin(person))
                .expect("the mirrored network administrator");
        }

        npc_ids.insert(npc.user.clone(), person);
    }

    if let Some(traffic) = &snapshot.traffic {
        let router = block_on(
            world
                .modules
                .creation
                .network_generation
                .router_or_generate(&snapshot.public_ip),
        )
        .expect("the police router is read")
        .expect("the mirrored police router");
        let network = block_on(world.modules.core.networks.routers.router(router))
            .expect("the police network is read")
            .expect("the mirrored police network")
            .network;
        let credentials = npc_ids.get(&*traffic.credentials_user).copied();
        let initial = TrafficNet {
            credentials,
            override_enabled: traffic.override_enabled,
            override_used: false,
        };

        block_on(
            world
                .modules
                .smart_devices
                .traffic
                .save_traffic(network, &initial),
        )
        .expect("the mirrored traffic state");

        if traffic.override_used {
            if let Some(accident_at) = &traffic.accident_at {
                world.modules.routines.set_date(accident_at);
            }

            let npc = snapshot
                .npcs
                .iter()
                .find(|npc| Some(&npc.user) == traffic.accident_driver.as_ref())
                .expect("the exact accident driver");
            let credentials_npc = snapshot
                .npcs
                .iter()
                .find(|npc| npc.user == traffic.credentials_user)
                .expect("the traffic credentials NPC");
            let workstation = block_on(
                world
                    .modules
                    .core
                    .networks
                    .routers
                    .computer_at(&snapshot.public_ip, &credentials_npc.local_ip),
            )
            .expect("the traffic workstation is read")
            .expect("the traffic workstation");
            let password = block_on(
                world
                    .modules
                    .core
                    .computers
                    .users
                    .user(workstation, &traffic.credentials_user),
            )
            .expect("the traffic account is read")
            .expect("the traffic account")
            .password;

            block_on(world.modules.smart_devices.traffic.locate_vehicle(
                network,
                &npc.license_plate,
                &password,
                true,
            ))
            .expect("the mirrored accident vehicle is located")
            .expect("the mirrored accident vehicle is on camera");
            assert!(
                block_on(world.modules.smart_devices.traffic.traffic_accident())
                    .expect("the mirrored traffic accident")
            );
            world.modules.routines.set_date(&snapshot.current_date);
        }

        let state = TrafficNet {
            credentials,
            override_enabled: traffic.override_enabled,
            override_used: traffic.override_used,
        };

        block_on(
            world
                .modules
                .smart_devices
                .traffic
                .save_traffic(network, &state),
        )
        .expect("the final mirrored traffic state");
    }

    if let Some(mail) = &snapshot.player_mail {
        let generated = block_on(world.modules.npcs.mail.mail_account(&mail.address))
            .expect("the generated mail account")
            .expect("the player's mail account");

        // World generation includes the ordinary welcome message. Replace it with the inbox the game actually has.
        for email in generated.inbox {
            block_on(
                world
                    .modules
                    .npcs
                    .mail
                    .delete_mail(&mail.address, &email.id),
            )
            .expect("the generated mail is removed");
        }

        // The game lists the newest mail first, as the world does.
        for email in mail.emails.iter().rev() {
            block_on(world.modules.npcs.mail.deliver_mail(
                &mail.address,
                &email.from,
                &email.subject,
                &email.body,
            ))
            .expect("the mirrored mail is delivered");
        }
    }

    if let Some(mail) = &snapshot.recipient_mail {
        block_on(
            world
                .modules
                .npcs
                .mail
                .add_mail_account(&mail.address, &mail.password),
        )
        .expect("the recipient mail account");
        for email in mail.emails.iter().rev() {
            block_on(world.modules.npcs.mail.deliver_mail(
                &mail.address,
                &email.from,
                &email.subject,
                &email.body,
            ))
            .expect("the recipient mail is mirrored");
        }
    }

    for mail in snapshot
        .player_mail
        .iter()
        .chain(snapshot.recipient_mail.iter())
        .chain(snapshot.npcs.iter().filter_map(|npc| npc.mail.as_ref()))
    {
        block_on(world.modules.npcs.mail.set_spam_config(
            &mail.address,
            MailSpamConfig {
                player_pc_id: mail.player_pc_id.clone(),
                filter: mail.spam_filter,
                blacklist: mail.blacklist.clone(),
            },
        ))
        .expect("the mirrored mail spam state");
    }

    block_on(
        world
            .modules
            .economy
            .bank
            .mirror_economy(&economy(snapshot)),
    )
    .expect("the mirrored economy");

    world
}

/// Runs two programs concurrently in the same mirrored world, returning each program's terminal and runtime error.
pub fn run_pair_in_snapshot(
    first: &str,
    second: &str,
    user: &str,
    snapshot: &NetworkSnapshot,
) -> [(String, Option<String>); 2] {
    let world = snapshot_world(user, snapshot);

    duplex_test_support::local::run_local(async move {
        let server = WorldServer::with_database(
            world,
            Database::in_memory().await.expect("an in-memory database"),
            constant_clock(),
        )
        .await
        .expect("the world is written to the database");
        let first_stdout = Rc::new(MarkedStdout::default());
        let second_stdout = Rc::new(MarkedStdout::default());
        let options = |stdout: &Rc<MarkedStdout>| InterpreterOptions {
            stdout: Rc::clone(stdout) as Rc<dyn Stdout>,
            ..InterpreterOptions::new(Rc::new(scheduler::TokioScheduler::new()))
        };
        let (first_error, second_error) = tokio::join!(
            run_in(&server, first, user, true, options(&first_stdout)),
            run_in(&server, second, user, true, options(&second_stdout)),
        );

        [
            (first_stdout.terminal_text(), first_error),
            (second_stdout.terminal_text(), second_error),
        ]
    })
}

/// The mails of a game mailbox as the world stores them, newest first.
fn mail_emails(emails: &[MailSnapshot]) -> Vec<Email> {
    emails
        .iter()
        .map(|email| Email {
            id: email.id.clone(),
            from: email.from.clone(),
            subject: email.subject.clone(),
            body: email.body.clone(),
            replies: email.replies.iter().map(ToString::to_string).collect(),
        })
        .collect()
}
