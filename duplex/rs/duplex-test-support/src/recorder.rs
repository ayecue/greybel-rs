//! Shared handler-test value recorder.

use std::cell::RefCell;
use std::rc::Rc;

/// Returns a shared vector and cloneable appending callback for use as a handler or inside a wrapper closure.
pub fn recorder<T: 'static>() -> (Rc<RefCell<Vec<T>>>, impl Fn(T) + Clone + 'static) {
    let values = Rc::new(RefCell::new(Vec::new()));
    let record = {
        let values = Rc::clone(&values);

        move |value| values.borrow_mut().push(value)
    };

    (values, record)
}
