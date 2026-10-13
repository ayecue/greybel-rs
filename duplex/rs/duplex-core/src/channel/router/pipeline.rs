use std::cell::RefCell;
use std::rc::Rc;

pub(crate) type HandlerContainer<H> = Rc<[Rc<H>]>;

pub(crate) struct Pipeline<H: ?Sized>(RefCell<HandlerContainer<H>>);

impl<H: ?Sized> Default for Pipeline<H> {
    fn default() -> Self {
        Self(RefCell::new(Rc::new([])))
    }
}

impl<H: ?Sized> Pipeline<H> {
    pub(crate) fn add(&self, handler: Rc<H>) {
        let mut handlers = self.0.borrow_mut();

        *handlers = handlers
            .iter()
            .cloned()
            .chain(std::iter::once(handler))
            .collect();
    }

    pub(crate) fn handlers(&self) -> HandlerContainer<H> {
        Rc::clone(&self.0.borrow())
    }
}
