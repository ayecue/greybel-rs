//! Conformance calculator over stdin/stdout for TypeScript package tests.

use duplex_core::{Channel, ChannelOptions, NextMessage, Responder};
use duplex_driver_stdio::StreamDriver;
use duplex_test_support::conformance::migration::{CalculationMigration1 as _, Upgrade};
use duplex_test_support::conformance::{CalculatorService, calculator_v2 as v2};
use duplex_test_support::local::run_local;
use tokio::task;
use v2::{Calculator, CalculatorRouter, Confirm, Divide, DivisionFailure, Quotient};

/// Division handler registered before the calculator's default handler; requests user confirmation before computing
/// the quotient.
struct ConfirmedDivision(Channel<Calculator, StreamDriver>);

impl CalculatorRouter for ConfirmedDivision {
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

        let Ok(confirmation) = self.0.send(&Confirm {
            question: format!("divide {} by {}?", message.dividend, message.divisor).into(),
        }) else {
            return;
        };

        task::spawn_local(async move {
            match confirmation.await {
                Ok(true) => responder.respond(Quotient::WithRemainder {
                    value: message.dividend / message.divisor,
                    remainder: message.dividend % message.divisor,
                }),
                Ok(false) => responder.fail(DivisionFailure::Refusal),
                Err(_) => drop(responder),
            }
        });
    }
}

fn main() {
    run_local(async {
        let transport = StreamDriver::stdio();
        let channel = Channel::<Calculator, _>::new(&transport, ChannelOptions::default());

        Upgrade.attach(&channel);
        ConfirmedDivision(channel.clone()).attach(&channel);
        CalculatorService(channel.clone()).attach(&channel);
        channel.close_event().await;
    });
}
