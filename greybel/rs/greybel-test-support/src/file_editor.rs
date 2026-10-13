//! In-memory editor for the documents protocol.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greybel_documents_host_provider::protocol::{Editor, FindFiles, ReadFile};

/// Serves URI-addressed files and logs every `ReadFile` URI.
pub struct FileEditor {
    files: BTreeMap<String, String>,
    reads: RefCell<Vec<String>>,
}

impl FileEditor {
    pub fn serve<D: Driver>(channel: &Channel<Editor, D>, files: &[(&str, &str)]) -> Rc<Self> {
        let editor = Rc::new(Self {
            files: files
                .iter()
                .map(|(uri, content)| ((*uri).to_owned(), (*content).to_owned()))
                .collect(),
            reads: RefCell::new(Vec::new()),
        });

        channel.handle({
            let editor = Rc::clone(&editor);

            move |read: ReadFile<'static>, response, _next| {
                editor.reads.borrow_mut().push(read.uri.to_string());
                response.respond(editor.files.get(read.uri.as_ref()).cloned());
            }
        });
        channel.handle({
            let editor = Rc::clone(&editor);

            move |_: FindFiles<'static>, response, _next| {
                response.respond(editor.files.keys().cloned().collect());
            }
        });

        editor
    }

    pub fn reads(&self) -> Vec<String> {
        self.reads.borrow().clone()
    }
}
