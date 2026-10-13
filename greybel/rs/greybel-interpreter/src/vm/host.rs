use std::rc::Rc;
use std::time::Duration;

use ::scheduler::Clock;
use greybel_io::{Stdin, Stdout};

use super::context::{ContextStack, HostData};

pub(super) struct HostContext {
    pub data: HostData,
    pub context_stack: Box<ContextStack>,
    pub stdout: Rc<dyn Stdout>,
    pub stdin: Rc<dyn Stdin>,
    pub clock: Rc<dyn Clock>,
    pub start_time: Duration,
}

impl HostContext {
    pub fn new(stdout: Rc<dyn Stdout>, stdin: Rc<dyn Stdin>, clock: Rc<dyn Clock>) -> Self {
        let start_time = clock.now();

        Self {
            data: HostData::default(),
            context_stack: Box::default(),
            stdout,
            stdin,
            clock,
            start_time,
        }
    }
}
