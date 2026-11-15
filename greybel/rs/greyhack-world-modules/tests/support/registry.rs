use greyhack_world_modules::Dep;

pub(crate) trait Call {
    fn call(&self) -> u32;
}

pub(crate) struct Provider;

impl Call for Provider {
    fn call(&self) -> u32 {
        42
    }
}

pub(crate) struct FirstService {
    pub(crate) other: Dep<SecondService>,
}

impl FirstService {
    pub(crate) fn base() -> u32 {
        20
    }

    pub(crate) fn call(&self) -> u32 {
        self.other.call() + 1
    }
}

pub(crate) struct SecondService {
    pub(crate) other: Dep<FirstService>,
}

impl SecondService {
    pub(crate) fn call(&self) -> u32 {
        // Resolve the reverse dependency before using the stateless base value.
        let _ = &*self.other;

        FirstService::base() + 21
    }
}
