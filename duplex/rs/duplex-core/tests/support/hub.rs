use std::cell::RefCell;
use std::rc::Rc;

use duplex_core::{Bundle, Channel, ChannelOptions};
use duplex_test_support::conformance::calculator::{Add, Calculator, Log};
use duplex_test_support::{Memory, recorder};

pub fn keyed(key: u32) -> ChannelOptions {
    ChannelOptions {
        bundle: Bundle::Key(key),
        ..ChannelOptions::default()
    }
}

pub fn counting(calculator: &Channel<Calculator, Memory>) -> Rc<RefCell<Vec<String>>> {
    let (handled, record) = recorder();

    calculator.on::<Log>({
        let record = record.clone();

        move |log, _next| record(log.text.to_string())
    });
    calculator.handle(move |add: Add, response, _next| {
        record(format!("{}+{}", add.a, add.b));
        response.respond(add.a + add.b);
    });

    handled
}
