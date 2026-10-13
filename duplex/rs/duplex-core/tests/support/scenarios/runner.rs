use std::cell::RefCell;
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::pin;
use std::rc::Rc;
use std::task::Poll;
use std::time::Duration;

use duplex_core::{
    Channel, ChannelOptions, ConnectionId, ConnectionState, Driver, Error, Health, HealthCheck,
    HealthChecks, Limits, PendingResponse, ResponseStream, Role, Security, SecurityRole,
    StreamWriter, Transport,
};
use duplex_test_support::conformance::calculator::{
    Calculator, DivisionFailure, Phase, Quotient, User,
};
use duplex_test_support::conformance::migration::{CalculationMigration1, Upgrade};
use duplex_test_support::conformance::{
    calculator_v2 as v2, count_to, login, sign_in, sum_terms, vault,
};
use duplex_test_support::frames::{AliasTable, Slice};
use duplex_test_support::future::poll;
use duplex_test_support::{ManualScheduler, TaskQueue};
use serde_json::{Value, json};

use super::values::{CalculatorMessage, Result, Subject, batch, code, quotient_json};
use crate::support::security::zeros;

#[derive(Clone, Default)]
pub struct Recorder(Rc<Recorded>);

#[derive(Default)]
struct Recorded {
    sent: RefCell<Vec<Vec<u8>>>,
    spawned: TaskQueue,
    scheduler: ManualScheduler,
}

impl Recorder {
    fn spawn(&self, future: impl Future<Output = ()> + 'static) {
        self.0.spawned.spawn(future);
    }

    fn run_tasks_once(&self) {
        self.0.scheduler.run_tasks_once();
    }

    fn run_tasks(&self) {
        loop {
            while self.0.spawned.poll() {}

            if !self.0.scheduler.run_task() {
                return;
            }
        }
    }

    fn take_sent(&self) -> Vec<Vec<u8>> {
        self.run_tasks();

        std::mem::take(&mut *self.0.sent.borrow_mut())
    }
}

impl Driver for Recorder {
    type Scheduler = ManualScheduler;

    const CAPABILITIES: duplex_core::Capabilities = duplex_core::Capabilities::DUPLEX;

    fn transport_name(&self) -> &'static str {
        "memory"
    }

    fn send(&self, batch: duplex_core::OutboundBatch) {
        self.0.sent.borrow_mut().push(batch.into_vec());
    }

    fn scheduler(&self) -> &ManualScheduler {
        &self.0.scheduler
    }

    fn random(&self) -> [u8; 16] {
        [0; 16]
    }
}

enum Peer {
    Client(Channel<User, Recorder>),
    Provider(Channel<Calculator, Recorder>),
    Migrating(Channel<v2::Calculator, Recorder>),
    Member(Channel<login::Member, Recorder>),
    Service(Channel<login::Service, Recorder>),
    Asker(Channel<vault::Asker, Recorder>),
    Keeper(Channel<vault::Keeper, Recorder>),
}

enum Pending {
    Add(PendingResponse<i64>),
    Divide(PendingResponse<Quotient, DivisionFailure>),
    Confirm(PendingResponse<bool>),
    Whoami(PendingResponse<String>),
    Reveal(PendingResponse<String>),
    Count(Counting),
    Sum {
        terms: Option<StreamWriter<(), i64>>,
        sum: PendingResponse<i64>,
    },
    Settled(Value),
}

struct Counting {
    stream: Option<ResponseStream<Phase, u64, u32>>,
    start: Option<Value>,
    items: Vec<Value>,
    ended: Option<Value>,
}

impl Counting {
    fn json(&self) -> Value {
        let mut outcome = json!({ "items": self.items });

        if let Some(start) = &self.start {
            outcome["start"] = start.clone();
        }

        if let Some(Value::Object(ended)) = &self.ended {
            for (key, value) in ended {
                outcome[key] = value.clone();
            }
        }

        outcome
    }
}

struct Remote {
    _peer: Peer,
    transport: Transport<Recorder>,
    driver: Recorder,
    forge: bool,
}

struct Build<'a> {
    scenario: &'a Value,
    subject: Subject,
    options: ChannelOptions,
    unhandled: Vec<&'a str>,
    received: Rc<RefCell<Vec<Value>>>,
}

pub struct Scenario {
    peer: Peer,
    remote: Option<Remote>,
    transport: Transport<Recorder>,
    aliases: AliasTable,
    subject: Subject,
    driver: Recorder,
    received: Rc<RefCell<Vec<Value>>>,
    deprecated: Rc<RefCell<Vec<Value>>>,
    requests: BTreeMap<String, Pending>,
}

macro_rules! on_peer {
    ($peer:expr, $channel:ident => $body:expr) => {
        match $peer {
            Peer::Client($channel) => $body,
            Peer::Provider($channel) => $body,
            Peer::Migrating($channel) => $body,
            Peer::Member($channel) => $body,
            Peer::Service($channel) => $body,
            Peer::Asker($channel) => $body,
            Peer::Keeper($channel) => $body,
        }
    };
}

impl Build<'_> {
    fn channel(
        &self,
        role: Role,
        transport: &Transport<Recorder>,
        driver: &Recorder,
    ) -> Result<Peer> {
        let (options, unhandled, received) = (self.options, &self.unhandled, &self.received);
        let major = self
            .scenario
            .get("major")
            .and_then(Value::as_u64)
            .unwrap_or(1);

        Ok(match (role, self.subject, major) {
            (Role::Client, Subject::Login, _) => {
                let open = Subject::credentials(&self.scenario["open"])?;
                let channel = Channel::<login::Member, _>::open(transport, options, &open)
                    .map_err(|error| error.to_string())?;

                Peer::Member(channel)
            }
            (Role::Provider, Subject::Login, _) => {
                let channel = Channel::<login::Service, _>::new(transport, options);

                Scenario::serve_members(&channel, unhandled);

                Peer::Service(channel)
            }
            (Role::Client, Subject::Vault, _) => {
                Peer::Asker(Channel::<vault::Asker, _>::new(transport, options))
            }
            (Role::Provider, Subject::Vault, _) => {
                let channel = Channel::<vault::Keeper, _>::new(transport, options);

                channel.handle(|reveal: vault::Reveal<'static>, response, _next| {
                    response.respond(format!("the {}", reveal.name));
                });

                Peer::Keeper(channel)
            }
            (Role::Provider, Subject::Calculator, 2) => {
                let channel = Channel::<v2::Calculator, _>::new(transport, options);

                if self.scenario["migrates"].as_bool().unwrap_or(true) {
                    Upgrade.attach(&channel);
                }

                Scenario::handle_calculations_of_2(&channel, driver, unhandled, received);

                Peer::Migrating(channel)
            }
            (Role::Client, Subject::Calculator, 1) => {
                let channel = Channel::<User, _>::new(transport, options);

                if !unhandled.contains(&"Confirm") {
                    channel.handle(
                        |confirm: duplex_test_support::conformance::calculator::Confirm<
                            'static,
                        >,
                         response,
                         _next| {
                            response.respond(confirm.question.ends_with('?'));
                        },
                    );
                }

                if !unhandled.contains(&"Progress") {
                    let received = Rc::clone(received);

                    channel.on::<duplex_test_support::conformance::calculator::Progress>(
                        move |progress, _next| {
                            received
                                .borrow_mut()
                                .push(CalculatorMessage::progress_json(&progress.into_owned()));
                        },
                    );
                }

                Peer::Client(channel)
            }
            (Role::Provider, Subject::Calculator, 1) => {
                let channel = Channel::<Calculator, _>::new(transport, options);

                Scenario::handle_calculations(&channel, driver, unhandled, received);

                Peer::Provider(channel)
            }
            _ => return Err("no such peer".to_owned()),
        })
    }

    fn transport(&self, role: Role, end: &str, driver: &Recorder) -> Result<Transport<Recorder>> {
        let driver = driver.clone();
        let Some(security) = self.scenario.get("security") else {
            return Ok(Transport::new(|_| driver));
        };
        let end = &security[end];
        let role = match role {
            Role::Client => SecurityRole::Initiator,
            Role::Provider => SecurityRole::Responder,
        };
        let mut security = Security::new(role).with_entropy(zeros);

        if let Some(identity) = end.get("identity") {
            security = security.with_identity(Self::key(identity)?);
        }

        if let Some(trusts) = end.get("trusts") {
            let keys = trusts
                .as_array()
                .ok_or("no identities")?
                .iter()
                .map(|identity| Self::key(identity).map(Security::public_key))
                .collect::<Result<_>>()?;

            security = security.with_pinned_keys(keys);
        }

        if end["signsBatches"].as_bool().unwrap_or(false) {
            security = security.with_batch_signatures();
        }

        Ok(Transport::with_security(|_| driver, security))
    }

    fn key(value: &Value) -> Result<[u8; 32]> {
        let byte = value
            .as_u64()
            .and_then(|byte| u8::try_from(byte).ok())
            .ok_or_else(|| format!("no identity {value}"))?;

        Ok([byte; 32])
    }
}

impl Scenario {
    pub fn run(scenario: &Value) -> Result<()> {
        let mut run = Self::start(scenario)?;

        for (index, step) in scenario["steps"]
            .as_array()
            .ok_or("no steps")?
            .iter()
            .enumerate()
        {
            run.step(step)
                .map_err(|error| format!("step {index}, {step}:\n{error}"))?;
        }

        Ok(())
    }

    fn start(scenario: &Value) -> Result<Self> {
        let checked = scenario["healthCheck"].as_bool().unwrap_or(false);
        let limit = |name: &str, default: usize| {
            scenario["limits"][name].as_u64().map_or(default, |limit| {
                usize::try_from(limit).unwrap_or(usize::MAX)
            })
        };
        let options = ChannelOptions {
            health_check: Some(if checked {
                HealthChecks::On(HealthCheck::DEFAULT)
            } else {
                HealthChecks::Off
            }),
            limits: Limits {
                queued_items: limit("queued", Limits::DEFAULT.queued_items),
                pending_items: limit("pending", Limits::DEFAULT.pending_items),
                active_responses: limit("responding", Limits::DEFAULT.active_responses),
                incoming_items: limit("waiting", Limits::DEFAULT.incoming_items),
            },
            max_message: scenario["limits"]["message"]
                .as_u64()
                .and_then(|most| u32::try_from(most).ok()),
            stream_window: scenario["streamWindow"]
                .as_u64()
                .and_then(|window| u32::try_from(window).ok()),
            unvalidated: scenario["unvalidated"].as_bool().unwrap_or(false),
            ..ChannelOptions::default()
        };
        let build = Build {
            scenario,
            subject: Subject::of(scenario)?,
            options,
            unhandled: scenario["unhandled"]
                .as_array()
                .map(|names| names.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default(),
            received: Rc::new(RefCell::new(Vec::new())),
        };
        let role = match scenario["peer"].as_str() {
            Some("client") => Role::Client,
            Some("provider") => Role::Provider,
            _ => return Err("no such peer".to_owned()),
        };
        let driver = Recorder::default();
        let transport = build.transport(role, "channel", &driver)?;
        let peer = build.channel(role, &transport, &driver)?;
        let remote = match scenario.get("security") {
            Some(_) => {
                let driver = Recorder::default();
                let other = match role {
                    Role::Client => Role::Provider,
                    Role::Provider => Role::Client,
                };
                let transport = build.transport(other, "remote", &driver)?;
                let remote = Build {
                    scenario,
                    subject: build.subject,
                    options: build.options,
                    unhandled: build.unhandled.clone(),
                    received: Rc::default(),
                };
                let peer = remote.channel(other, &transport, &driver)?;

                Some(Remote {
                    _peer: peer,
                    transport,
                    driver,
                    forge: false,
                })
            }
            None => None,
        };

        driver.run_tasks_once();
        if let Some(remote) = &remote {
            remote.driver.run_tasks_once();
        }

        let deprecated = Rc::new(RefCell::new(Vec::new()));

        {
            let deprecated = Rc::clone(&deprecated);

            on_peer!(&peer, channel => channel.on_ack(move |active| {
                if let Some((version, note)) = active.deprecation() {
                    deprecated.borrow_mut().push(json!({
                        "version": format!("{}.{}", version.major, version.minor),
                        "note": note,
                    }));
                }
            }));
        }

        let mut run = Self {
            peer,
            remote,
            transport,
            aliases: AliasTable::default(),
            subject: build.subject,
            driver,
            received: build.received,
            deprecated,
            requests: BTreeMap::new(),
        };

        if scenario["connected"].as_bool().unwrap_or(false) {
            run.connect()?;
        }

        Ok(run)
    }

    fn handle_calculations(
        channel: &Channel<Calculator, Recorder>,
        driver: &Recorder,
        unhandled: &[&str],
        received: &Rc<RefCell<Vec<Value>>>,
    ) {
        use duplex_test_support::conformance::calculator::{Add, Count, Divide, Log, Sum};

        if !unhandled.contains(&"Count") {
            let driver = driver.clone();

            channel.handle(move |count: Count, responder, _next| {
                driver.spawn(count_to(count.to, Phase::Active, responder));
            });
        }

        if !unhandled.contains(&"Sum") {
            let driver = driver.clone();

            channel.handle(move |_: Sum, stream, _next| driver.spawn(sum_terms(stream)));
        }

        if !unhandled.contains(&"Log") {
            let received = Rc::clone(received);

            channel.on::<Log>(move |log, _next| {
                received
                    .borrow_mut()
                    .push(json!({ "Log": { "text": log.text } }));
            });
        }

        if !unhandled.contains(&"Add") {
            channel.handle(|add: Add, response, _next| response.respond(add.a + add.b));
        }

        if !unhandled.contains(&"Divide") {
            channel.handle(|divide: Divide, response, _next| {
                if divide.divisor == 0 {
                    response.fail(DivisionFailure::ByZero);

                    return;
                }

                let value = divide.dividend / divide.divisor;
                let remainder = divide.dividend % divide.divisor;

                response.respond(if remainder == 0 {
                    Quotient::Exact { value }
                } else {
                    Quotient::WithRemainder { value, remainder }
                });
            });
        }
    }

    fn serve_members(channel: &Channel<login::Service, Recorder>, unhandled: &[&str]) {
        if !unhandled.contains(&"Open") {
            channel.on_connect(sign_in);
        }

        if !unhandled.contains(&"Whoami") {
            channel.handle(|_: login::Whoami, response, _next| response.respond("ada".to_owned()));
        }
    }

    fn handle_calculations_of_2(
        channel: &Channel<v2::Calculator, Recorder>,
        driver: &Recorder,
        unhandled: &[&str],
        received: &Rc<RefCell<Vec<Value>>>,
    ) {
        if !unhandled.contains(&"Count") {
            let driver = driver.clone();

            channel.handle(move |count: v2::Count, responder, _next| {
                driver.spawn(count_to(count.to, v2::Phase::Active, responder));
            });
        }

        if !unhandled.contains(&"Sum") {
            let driver = driver.clone();

            channel.handle(move |_: v2::Sum, stream, _next| driver.spawn(sum_terms(stream)));
        }

        if !unhandled.contains(&"Log") {
            let received = Rc::clone(received);

            channel.on::<v2::Log>(move |log, _next| {
                received
                    .borrow_mut()
                    .push(json!({ "Log": { "text": log.text } }));
            });
        }

        if !unhandled.contains(&"Add") {
            channel.handle(|add: v2::Add<'static>, response, _next| {
                response.respond(add.terms.iter().sum());
            });
        }

        if !unhandled.contains(&"Divide") {
            channel.handle(|divide: v2::Divide, response, _next| {
                if divide.divisor == 0 {
                    response.fail(v2::DivisionFailure::ByZero);

                    return;
                }

                let value = divide.dividend / divide.divisor;
                let remainder = divide.dividend % divide.divisor;

                response.respond(if remainder == 0 {
                    v2::Quotient::Exact { value }
                } else {
                    v2::Quotient::WithRemainder { value, remainder }
                });
            });
        }
    }

    fn role(&self) -> Role {
        match self.peer {
            Peer::Client(_) | Peer::Member(_) | Peer::Asker(_) => Role::Client,
            Peer::Provider(_) | Peer::Migrating(_) | Peer::Service(_) | Peer::Keeper(_) => {
                Role::Provider
            }
        }
    }

    fn remote(&self) -> Role {
        match self.role() {
            Role::Client => Role::Provider,
            Role::Provider => Role::Client,
        }
    }

    fn connect(&mut self) -> Result<()> {
        if self.remote.is_some() {
            self.pass();

            return match on_peer!(&self.peer, channel => channel.connection()) {
                Some(_) => Ok(()),
                None => Err("didn't connect".to_owned()),
            };
        }

        self.discard_sent();

        let frames = match (self.role(), self.subject) {
            (Role::Client, Subject::Login) => json!([{
                "Ack": { "id": 0, "connection": 1, "payload": { "greeting": "hello, ada" } }
            }]),
            (Role::Provider, Subject::Login) => json!([{
                "Open": { "id": 0, "payload": { "user": "ada", "token": "t0k" } }
            }]),
            (Role::Client, _) => json!([{ "Ack": { "id": 0, "connection": 1 } }]),
            (Role::Provider, _) => json!([{ "Open": { "id": 0 } }]),
        };

        self.receive(&json!({ "connection": 0, "frames": frames }))?;
        self.discard_sent();

        match on_peer!(&self.peer, channel => channel.connection()) {
            Some(_) => Ok(()),
            None => Err("didn't connect".to_owned()),
        }
    }

    fn pass(&mut self) {
        let Some(remote) = &mut self.remote else {
            return;
        };

        loop {
            let sent = self.driver.take_sent();

            for batch in &sent {
                remote.transport.downgrade().receive(batch.clone());
            }

            let mut back = remote.driver.take_sent();

            if remote.forge
                && let Some(last) = back.first_mut().and_then(|batch| batch.last_mut())
            {
                *last ^= 1;
                remote.forge = false;
            }

            for batch in &back {
                self.transport.downgrade().receive(batch.clone());
            }

            if sent.is_empty() && back.is_empty() {
                return;
            }
        }
    }

    fn discard_sent(&mut self) {
        for batch in self.driver.take_sent() {
            self.aliases.read(&batch);
        }
    }

    fn receive(&self, value: &Value) -> Result<()> {
        let bytes = batch(value, self.remote(), self.subject)?;

        self.transport.downgrade().receive(bytes);

        Ok(())
    }

    fn step(&mut self, step: &Value) -> Result<()> {
        if let Some(value) = step.get("receive") {
            return self.receive(value);
        }

        if let Some(message) = step.get("send") {
            let Some(name) = step.get("as") else {
                return self.notify(message, step.get("fails"));
            };
            let name = name.as_str().ok_or("no name")?.to_owned();
            let pending = self.request(message)?;

            self.requests.insert(name, pending);

            return Ok(());
        }

        if let Some(count) = step.get("take") {
            let name = step["of"].as_str().ok_or("no name")?;

            return self.take(name, count.as_u64().ok_or("no count")?);
        }

        if let Some(name) = step.get("drop") {
            let name = name.as_str().ok_or("no name")?;

            self.requests
                .remove(name)
                .ok_or_else(|| format!("no message `{name}`"))?;

            return Ok(());
        }

        if let Some(item) = step.get("item") {
            let name = step["into"].as_str().ok_or("no name")?;

            return self.item(name, item);
        }

        if let Some(name) = step.get("end") {
            let name = name.as_str().ok_or("no name")?;
            let Some(Pending::Sum { terms, .. }) = self.requests.get_mut(name) else {
                return Err(format!("`{name}` streams nothing"));
            };
            let terms = terms.take().ok_or("the stream ended")?;

            return match poll(&mut pin!(terms.finish())) {
                Poll::Ready(ended) => ended.map_err(|error| error.to_string()),
                Poll::Pending => Err("the message wasn't sent".to_owned()),
            };
        }

        if let Some(checks) = step.get("check") {
            for _ in 0..checks.as_u64().ok_or("no count")? {
                on_peer!(&self.peer, channel => channel.check());
            }

            return Ok(());
        }

        if let Some(reason) = step.get("close") {
            let reason = code(reason)?;

            on_peer!(&self.peer, channel => channel.close(reason));

            return Ok(());
        }

        if step.get("forge").is_some() {
            let remote = self
                .remote
                .as_mut()
                .ok_or("no remote channel to forge for")?;

            remote.forge = true;

            return Ok(());
        }

        if let Some(wait) = step.get("wait") {
            let wait = wait.as_u64().ok_or("no milliseconds")?;

            self.driver.0.scheduler.advance(Duration::from_millis(wait));

            return Ok(());
        }

        if step.get("disconnect").is_some() {
            self.transport.downgrade().on_disconnect();

            return Ok(());
        }

        if let Some(expectation) = step.get("expect") {
            return self.expect(expectation);
        }

        Err("no step".to_owned())
    }

    fn notify(&self, message: &Value, fails: Option<&Value>) -> Result<()> {
        let notified = match (&self.peer, CalculatorMessage::of(message)?) {
            (Peer::Client(channel), CalculatorMessage::Log(log)) => channel.send(&log),
            (Peer::Client(channel), CalculatorMessage::Reset(reset)) => channel.send(&reset),
            (Peer::Provider(channel), CalculatorMessage::Progress(progress)) => {
                channel.send(&progress)
            }
            (Peer::Migrating(channel), CalculatorMessage::Progress(progress)) => {
                channel.send(&Self::progress_of_2(&progress))
            }
            _ => return Err(format!("the peer doesn't send {message}")),
        };
        let outcome = match notified {
            Ok(()) => None,
            Err(error) => Some(Self::error_json(&error)),
        };

        match (outcome, fails) {
            (None, None) => Ok(()),
            (Some(outcome), Some(fails)) if outcome == *fails => Ok(()),
            (outcome, fails) => Err(format!("notified: {outcome:?}, expected {fails:?}")),
        }
    }

    fn progress_of_2(
        progress: &duplex_test_support::conformance::calculator::Progress<'static>,
    ) -> v2::Progress<'static> {
        let phase = match progress.phase {
            duplex_test_support::conformance::calculator::Phase::Pending => v2::Phase::Pending,
            duplex_test_support::conformance::calculator::Phase::Active => v2::Phase::Active,
            duplex_test_support::conformance::calculator::Phase::Final => v2::Phase::Final,
        };

        v2::Progress {
            phase,
            done_share: None,
            steps: Vec::new().into(),
            counts: BTreeMap::new(),
            note: None,
            eta: progress.eta,
        }
    }

    fn request(&self, message: &Value) -> Result<Pending> {
        fn pending<T>(
            sent: std::result::Result<T, Error>,
            wrap: impl FnOnce(T) -> Pending,
        ) -> Pending {
            match sent {
                Ok(sent) => wrap(sent),
                Err(error) => Pending::Settled(Scenario::error_json(&error)),
            }
        }

        if let Peer::Asker(channel) = &self.peer {
            let (name, fields) = super::values::entry(message)?;

            return match name {
                "Reveal" => Ok(pending(
                    channel.send(&vault::Reveal {
                        name: fields["name"].as_str().ok_or("no name")?.to_owned().into(),
                    }),
                    Pending::Reveal,
                )),
                _ => Err(format!("the peer doesn't send {message}")),
            };
        }

        if let Peer::Member(channel) = &self.peer {
            return match super::values::entry(message)?.0 {
                "Whoami" => Ok(pending(channel.send(&login::Whoami {}), Pending::Whoami)),
                _ => Err(format!("the peer doesn't send {message}")),
            };
        }

        Ok(match (&self.peer, CalculatorMessage::of(message)?) {
            (Peer::Client(channel), CalculatorMessage::Add(add)) => {
                pending(channel.send(&add), Pending::Add)
            }
            (Peer::Client(channel), CalculatorMessage::Divide(divide)) => {
                pending(channel.send(&divide), Pending::Divide)
            }
            (Peer::Client(channel), CalculatorMessage::Count(count)) => {
                pending(channel.send(&count), |stream| {
                    Pending::Count(Counting {
                        stream: Some(stream),
                        start: None,
                        items: Vec::new(),
                        ended: None,
                    })
                })
            }
            (Peer::Client(channel), CalculatorMessage::Sum(sum)) => {
                pending(channel.send(&sum), |(terms, sum)| Pending::Sum {
                    terms: Some(terms),
                    sum,
                })
            }
            (Peer::Provider(channel), CalculatorMessage::Confirm(confirm)) => {
                pending(channel.send(&confirm), Pending::Confirm)
            }
            (Peer::Migrating(channel), CalculatorMessage::Confirm(confirm)) => pending(
                channel.send(&v2::Confirm {
                    question: confirm.question,
                }),
                Pending::Confirm,
            ),
            _ => return Err(format!("the peer doesn't send {message}")),
        })
    }

    fn take(&mut self, name: &str, count: u64) -> Result<()> {
        let Some(Pending::Count(counting)) = self.requests.get_mut(name) else {
            return Err(format!("`{name}` is no stream"));
        };
        let stream = counting.stream.as_mut().ok_or("the stream ended")?;

        if counting.start.is_none() {
            let started = poll(&mut pin!(stream.start()));

            match started {
                Poll::Ready(Ok(phase)) => counting.start = Some(super::values::phase_json(phase)),
                Poll::Ready(Err(error)) => {
                    counting.ended = Some(Self::error_json(&error));
                    counting.stream = None;

                    return Ok(());
                }
                Poll::Pending => return Err("the stream hasn't started".to_owned()),
            }
        }

        for _ in 0..count {
            let Some(stream) = counting.stream.as_mut() else {
                return Err("the stream ended".to_owned());
            };
            let next = poll(&mut pin!(stream.next()));

            match next {
                Poll::Ready(Some(Ok(item))) => counting.items.push(json!(item)),
                Poll::Ready(Some(Err(error))) => {
                    counting.ended = Some(Self::error_json(&error));
                    counting.stream = None;
                }
                Poll::Ready(None) => {
                    let stream = counting.stream.take().ok_or("the stream ended")?;

                    counting.ended = Some(match poll(&mut pin!(stream.end())) {
                        Poll::Ready(Ok(sum)) => json!({ "end": sum }),
                        Poll::Ready(Err(error)) => Self::error_json(&error),
                        Poll::Pending => return Err("the stream didn't end".to_owned()),
                    });
                }
                Poll::Pending => return Err(format!("no item {} yet", counting.items.len())),
            }
        }

        Ok(())
    }

    fn item(&mut self, name: &str, item: &Value) -> Result<()> {
        let Some(Pending::Sum {
            terms: Some(terms), ..
        }) = self.requests.get_mut(name)
        else {
            return Err(format!("`{name}` streams nothing"));
        };
        let term = item["Sum"].as_i64().ok_or("no term")?;

        match poll(&mut pin!(terms.send(&term))) {
            Poll::Ready(Ok(())) => Ok(()),
            Poll::Ready(Err(error)) => Err(error.to_string()),
            Poll::Pending => Err("the stream waits for credit".to_owned()),
        }
    }

    fn error_json<F: std::fmt::Debug>(error: &Error<F>) -> Value {
        match error {
            Error::Failure(failure) => json!({ "failed": format!("{failure:?}") }),
            Error::Rejection(code) => json!({ "rejected": format!("{code:?}") }),
            Error::ChannelEnd(code) => json!({ "closed": format!("{code:?}") }),
            Error::Encode(failure) => json!({ "error": failure.to_string() }),
            Error::Decode(failure) => json!({ "error": failure.to_string() }),
            Error::Invalid(violation) => json!({
                "invalid": { "path": violation.path, "requirement": format!("{:?}", violation.requirement) }
            }),
            Error::Unavailable => json!("unsupported"),
        }
    }

    fn outcome<V, F: std::fmt::Debug>(
        reply: &mut PendingResponse<V, F>,
        value: impl Fn(V) -> Value,
    ) -> Option<Value>
    where
        PendingResponse<V, F>: std::future::Future<Output = std::result::Result<V, Error<F>>>,
    {
        match poll(reply) {
            Poll::Pending => None,
            Poll::Ready(Ok(response)) => Some(json!({ "response": value(response) })),
            Poll::Ready(Err(error)) => Some(Self::error_json(&error)),
        }
    }

    fn settled(&mut self, name: &str) -> Result<Value> {
        let pending = self
            .requests
            .get_mut(name)
            .ok_or_else(|| format!("no message `{name}`"))?;
        let outcome = match pending {
            Pending::Settled(outcome) => return Ok(outcome.clone()),
            Pending::Add(reply) => Self::outcome(reply, |sum| json!(sum)),
            Pending::Divide(reply) => Self::outcome(reply, |quotient| quotient_json(&quotient)),
            Pending::Confirm(reply) => Self::outcome(reply, |confirmed| json!(confirmed)),
            Pending::Whoami(reply) | Pending::Reveal(reply) => {
                Self::outcome(reply, |text| json!(text))
            }
            Pending::Sum { sum, .. } => Self::outcome(sum, |sum| json!(sum)),
            Pending::Count(counting) => return Ok(counting.json()),
        };

        Ok(match outcome {
            Some(outcome) => {
                *pending = Pending::Settled(outcome.clone());

                outcome
            }
            None => json!("pending"),
        })
    }

    fn expect(&mut self, expectation: &Value) -> Result<()> {
        self.driver.run_tasks();
        self.pass();

        let mut failures = Vec::new();

        if let Some(sent) = expectation.get("sent") {
            if self.remote.is_some() {
                return Err("what a secure transport sends can't be read".to_owned());
            }

            let expected: Vec<Slice> = sent
                .as_array()
                .ok_or("no batches")?
                .iter()
                .map(|value| {
                    batch(value, self.role(), self.subject).map(|bytes| Slice::decode(&bytes))
                })
                .collect::<Result<_>>()?;
            let actual: Vec<Slice> = self
                .driver
                .take_sent()
                .iter()
                .flat_map(|bytes| self.aliases.read(bytes))
                .collect();

            if actual != expected {
                failures.push(format!("sent {actual:#?}\nexpected {expected:#?}"));
            }
        }

        if let Some(received) = expectation.get("received") {
            let actual = Value::Array(std::mem::take(&mut *self.received.borrow_mut()));

            if actual != *received {
                failures.push(format!("received {actual}, expected {received}"));
            }
        }

        if let Some(settled) = expectation.get("settled") {
            for (name, expected) in settled.as_object().ok_or("no requests")? {
                let actual = self.settled(name)?;

                if actual != *expected {
                    failures.push(format!("`{name}` settled {actual}, expected {expected}"));
                }
            }
        }

        if let Some(health) = expectation.get("health") {
            let actual = match on_peer!(&self.peer, channel => channel.health()) {
                Health::Healthy => "healthy",
                Health::Unhealthy => "unhealthy",
            };

            if health.as_str() != Some(actual) {
                failures.push(format!("health {actual}, expected {health}"));
            }
        }

        if let Some(deprecated) = expectation.get("deprecated") {
            let actual = Value::Array(std::mem::take(&mut *self.deprecated.borrow_mut()));

            if actual != *deprecated {
                failures.push(format!("deprecated {actual}, expected {deprecated}"));
            }
        }

        if let Some(version) = expectation.get("version") {
            let actual = on_peer!(&self.peer, channel => channel.version())
                .map_or(Value::Null, |version| {
                    json!(format!("{}.{}", version.major, version.minor))
                });

            if actual != *version {
                failures.push(format!("version {actual}, expected {version}"));
            }
        }

        if let Some(connection) = expectation.get("connection") {
            let actual =
                json!(on_peer!(&self.peer, channel => channel.connection().map(ConnectionId::get)));

            if actual != *connection {
                failures.push(format!("connection {actual}, expected {connection}"));
            }
        }

        if let Some(ack) = expectation.get("ack") {
            let actual = match &self.peer {
                Peer::Member(channel) => match channel.state() {
                    ConnectionState::Active(active) => active
                        .ack()
                        .map_or(Value::Null, |ack| json!({ "greeting": ack.greeting })),
                    _ => Value::Null,
                },
                _ => Value::Null,
            };

            if actual != *ack {
                failures.push(format!("ack {actual}, expected {ack}"));
            }
        }

        if let Some(refusal) = expectation.get("refusal") {
            let actual = match &self.peer {
                Peer::Member(channel) => match channel.state() {
                    ConnectionState::Failure(failed) => failed
                        .refusal()
                        .copied()
                        .map_or(Value::Null, Subject::denial_json),
                    _ => Value::Null,
                },
                _ => Value::Null,
            };

            if actual != *refusal {
                failures.push(format!("refusal {actual}, expected {refusal}"));
            }
        }

        if let Some(closed) = expectation.get("closed") {
            let actual = on_peer!(&self.peer, channel => channel.close_reason())
                .map_or(Value::Null, |code| json!(format!("{code:?}")));

            if actual != *closed {
                failures.push(format!("closed {actual}, expected {closed}"));
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("\n"))
        }
    }
}
