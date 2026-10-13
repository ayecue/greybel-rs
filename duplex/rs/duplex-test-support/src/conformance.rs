//! Conformance protocol bindings and reusable calculator and login test services.

use std::path::{Path, PathBuf};

use calculator::{Add, CalculatorRouter};
use duplex_core::{
    IncomingStream, NextMessage, PendingConnection, Responder, StreamResponder, ValidatedEncode,
};
use login::{Credentials, Denial, Login, ServiceRouter, Session, Whoami};

/// Calculator major 1 provider bindings.
pub mod calculator {
    duplex_core::include_protocol!("$DUPLEX/conformance/calculator", major = 1, role = provider);
}

/// Calculator major 1 client bindings.
pub mod user {
    duplex_core::include_protocol!("$DUPLEX/conformance/calculator", major = 1, role = client);
}

/// Calculator major 2 provider bindings.
pub mod calculator_v2 {
    duplex_core::include_protocol!("$DUPLEX/conformance/calculator", major = 2, role = provider);
}

/// Calculator major 1 to major 2 migration bindings.
pub mod migration {
    use duplex_core::ErrorCode;

    use super::{calculator as v1, calculator_v2 as v2};

    duplex_core::include_migration!(
        "$DUPLEX/conformance/calculator",
        from = 1,
        older = super::calculator,
        newer = super::calculator_v2
    );

    /// Converts two-operand major-1 addition to term-list major-2 addition.
    pub struct Upgrade;

    impl CalculationMigration1 for Upgrade {
        fn migrate_add(&self, message: v1::Add) -> Result<v2::Add<'static>, ErrorCode> {
            Ok(v2::Add {
                terms: vec![message.a, message.b].into(),
            })
        }
    }
}

/// Login protocol provider bindings.
pub mod login {
    duplex_core::include_protocol!("$DUPLEX/conformance/login", major = 1, role = provider);
}

/// Login protocol client bindings.
pub mod member {
    duplex_core::include_protocol!("$DUPLEX/conformance/login", major = 1, role = client);
}

pub mod vault {
    duplex_core::include_protocol!("$DUPLEX/conformance/vault", major = 1, role = provider);
}

pub mod legacy {
    duplex_core::include_protocol!("$DUPLEX/conformance/legacy", major = 1, role = provider);
}

pub mod sensor {
    duplex_core::include_protocol!("$DUPLEX/conformance/tags", major = 1, role = provider);
}

pub mod layouts {
    duplex_core::include_types!("$DUPLEX/conformance/layouts", major = 1);
}

/// Returns the path for conformance definition `name`.
#[must_use]
pub fn definition(name: &str) -> PathBuf {
    Path::new(env!("DUPLEX")).join("conformance").join(name)
}

/// Calculator handler summing both addition operands.
pub struct Adder;

impl CalculatorRouter for Adder {
    fn on_add(&self, request: Add, response: Responder<i64>, _next: NextMessage<Add>) {
        response.respond(request.a + request.b);
    }
}

/// Sends `start`, integers 1 through `to`, and their sum in a `Count` response.
pub async fn count_to<S: ValidatedEncode>(
    to: u32,
    start: S,
    responder: StreamResponder<S, u64, u32>,
) {
    let mut items = responder.start(start);

    for number in 1..=to {
        if items.send(number).await.is_err() {
            return;
        }
    }

    items.end((1..=u64::from(to)).sum());
}

/// Responds to `Sum` with the incoming term sum, returning early on stream failure.
pub async fn sum_terms((mut terms, responder): (IncomingStream<(), i64>, Responder<i64>)) {
    let mut sum = 0;

    while let Some(term) = terms.next().await {
        match term {
            Ok(term) => sum += term,
            Err(_) => return,
        }
    }

    responder.respond(sum);
}

/// Returns valid credentials for user `ada` and token `t0k`.
#[must_use]
pub fn credentials() -> Credentials {
    Credentials {
        user: "ada".to_owned(),
        token: "t0k".to_owned(),
    }
}

/// Accepts `ada` with token `t0k` and returns a greeting; rejects invalid tokens and unknown users.
#[expect(
    clippy::needless_pass_by_value,
    reason = "a connect handler takes the opening's payload"
)]
pub fn sign_in(credentials: Credentials, pending: PendingConnection<Login>) {
    match (credentials.user.as_str(), credentials.token.as_str()) {
        ("ada", "t0k") => pending.accept(Session {
            greeting: format!("hello, {}", credentials.user),
        }),
        ("ada", _) => pending.deny(Denial::WrongToken),
        _ => pending.deny(Denial::UnknownUser),
    }
}

/// Login service authenticating `ada` and returning `ada` for `Whoami`.
pub struct Directory;

impl ServiceRouter for Directory {
    fn on_connect(&self, open: Credentials, pending: PendingConnection<Login>) {
        sign_in(open, pending);
    }

    fn on_whoami(
        &self,
        _message: Whoami,
        responder: Responder<String>,
        _next: NextMessage<Whoami>,
    ) {
        responder.respond("ada".to_owned());
    }
}

#[cfg(feature = "tokio")]
pub use service::{CalculatorService, serve_calculator};

#[cfg(feature = "tokio")]
mod service {
    use std::collections::BTreeMap;

    use duplex_core::{
        Channel, ChannelOptions, Driver, IncomingStream, NextMessage, NextNotification, Responder,
        StreamResponder, Transport,
    };
    use tokio::task;

    use super::calculator_v2::{
        Add, Calculator, CalculatorRouter, Count, Divide, DivisionFailure, Log, LogRef, Phase,
        Progress, Quotient, Sum,
    };
    use super::migration::{CalculationMigration1 as _, Upgrade};
    use super::{count_to, sum_terms};

    /// Driver-independent major-2 calculator service. Emits `Progress` for each `Log` and runs streams on Tokio local
    /// tasks.
    pub struct CalculatorService<D: Driver>(pub Channel<Calculator, D>);

    impl<D: Driver> CalculatorRouter for CalculatorService<D> {
        fn on_log(&self, message: LogRef<'_>, _next: NextNotification<Log<'static>>) {
            let progress = Progress {
                phase: Phase::Active,
                done_share: None,
                steps: Vec::new().into(),
                counts: BTreeMap::new(),
                note: Some(Some(message.text.into())),
                eta: None,
            };
            let _ = self.0.send(&progress);
        }

        fn on_add(
            &self,
            message: Add<'static>,
            responder: Responder<i64>,
            _next: NextMessage<Add<'static>>,
        ) {
            responder.respond(message.terms.iter().sum());
        }

        fn on_divide(
            &self,
            message: Divide,
            responder: Responder<Quotient, DivisionFailure>,
            _next: NextMessage<Divide>,
        ) {
            if message.divisor == 0 {
                responder.fail(DivisionFailure::ByZero);

                return;
            }

            responder.respond(Quotient::WithRemainder {
                value: message.dividend / message.divisor,
                remainder: message.dividend % message.divisor,
            });
        }

        fn on_count(
            &self,
            message: Count,
            responder: StreamResponder<Phase, u64, u32>,
            _next: NextMessage<Count>,
        ) {
            task::spawn_local(count_to(message.to, Phase::Active, responder));
        }

        fn on_sum(
            &self,
            _message: Sum,
            stream: (IncomingStream<(), i64>, Responder<i64>),
            _next: NextMessage<Sum>,
        ) {
            task::spawn_local(sum_terms(stream));
        }
    }

    /// Serves calculator major 2 on `transport` until channel closure, migrating major 1 with [`Upgrade`].
    pub fn serve_calculator<D: Driver>(transport: Transport<D>) {
        let channel = Channel::<Calculator, _>::new(&transport, ChannelOptions::default());

        Upgrade.attach(&channel);
        CalculatorService(channel.clone()).attach(&channel);
        task::spawn_local(async move {
            channel.close_event().await;
            drop(transport);
        });
    }
}
