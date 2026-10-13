use greybel_core::Dialect;
use greybel_documents_memory_provider::MemoryProvider;
use greybel_interpreter::{Interpreter, InterpreterOptions};
use greybel_test_support::interpreter::test_api::TEST_API;
use greybel_test_support::interpreter::with_captured_stdout;
use greybel_test_support::io::PrintLog;

pub const MAIN: &str = "main";

pub fn interpreter(files: &[(&str, &str)]) -> (Interpreter<MemoryProvider>, PrintLog) {
    let (printed, options) = with_captured_stdout(InterpreterOptions {
        dialect: Dialect::GreyScript,
        target: MAIN.to_owned(),
        ..InterpreterOptions::new(std::rc::Rc::new(scheduler::ManualClock::constant()))
    });
    let interpreter = Interpreter::new(options, files.iter().copied().collect::<MemoryProvider>())
        .with_api(TEST_API);

    (interpreter, printed)
}
