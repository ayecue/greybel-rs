use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use duplex_core::{Channel, ChannelOptions, Error, Requirement, Violation};
use duplex_test_support::conformance::calculator::{
    Calculator, Divide, Phase, Progress, Quotient, Step, Sum, User,
};
use duplex_test_support::{Memory, MemoryQueue};

pub fn unvalidated() -> ChannelOptions {
    ChannelOptions {
        unvalidated: true,
        ..ChannelOptions::default()
    }
}

pub fn invalid(path: &[u32], requirement: Requirement) -> Error {
    Error::Invalid(Violation {
        path: path.to_vec(),
        requirement,
    })
}

pub fn connected(
    user: ChannelOptions,
    calculator: ChannelOptions,
) -> (
    Channel<User, Memory>,
    Channel<Calculator, Memory>,
    MemoryQueue,
) {
    let (user, calculator, queue) = Memory::between_with(user, calculator);

    queue.deliver();

    (user, calculator, queue)
}

pub fn dividing(calculator: &Channel<Calculator, Memory>) -> Rc<RefCell<u32>> {
    let handled = Rc::new(RefCell::new(0));

    calculator.handle({
        let handled = Rc::clone(&handled);

        move |divide: Divide, responder, _next| {
            *handled.borrow_mut() += 1;
            responder.respond(Quotient::Exact {
                value: divide.dividend / divide.divisor,
            });
        }
    });

    handled
}

pub fn divide(precision: u8) -> Divide {
    Divide {
        dividend: 6,
        divisor: 3,
        precision: Some(precision),
    }
}

pub fn step(name: &str) -> Step {
    Step {
        name: name.to_owned(),
        result: None,
        duration: None,
    }
}

pub fn progress(steps: Vec<Step>) -> Progress<'static> {
    Progress {
        phase: Phase::Active,
        done_share: None,
        steps: steps.into(),
        counts: BTreeMap::default(),
        note: None,
        eta: None,
    }
}

pub type Kept = Rc<RefCell<Vec<Box<dyn std::any::Any>>>>;

pub fn keeping_sums(calculator: &Channel<Calculator, Memory>) -> Kept {
    let kept: Kept = Rc::default();

    calculator.handle({
        let kept = Rc::clone(&kept);

        move |_: Sum, exchange, _next| kept.borrow_mut().push(Box::new(exchange))
    });

    kept
}
