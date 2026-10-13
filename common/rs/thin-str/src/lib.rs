//! Immutable reference-counted string using a single allocation.

#![allow(unsafe_code)]

use std::alloc::{self, Layout};
use std::cell::Cell;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::{self, NonNull};

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;

/// Allocation header immediately followed by UTF-8 string bytes.
#[repr(C)]
struct Inner<H> {
    strong: Cell<usize>,
    len: usize,
    header: H,
}

pub struct ThinStr<H: Copy> {
    inner: NonNull<Inner<H>>,

    marker: PhantomData<Inner<H>>,
}

impl<H: Copy> ThinStr<H> {
    const BYTES: usize = size_of::<Inner<H>>();

    #[inline(never)]
    #[must_use]
    pub fn new(header: H, text: &str) -> Self {
        Self::concat(header, &[text])
    }

    /// Allocates one string for concatenated `parts` and writes their bytes directly into that allocation.
    #[inline(never)]
    #[must_use]
    pub fn concat(header: H, parts: &[&str]) -> Self {
        let len = parts
            .iter()
            .try_fold(0_usize, |len, part| len.checked_add(part.len()))
            .expect("the parts fit in memory");
        let this = Self::allocate(header, len);
        let mut bytes = this.bytes();

        for part in parts {
            unsafe {
                ptr::copy_nonoverlapping(part.as_ptr(), bytes, part.len());
                bytes = bytes.add(part.len());
            }
        }

        this
    }

    fn layout(len: usize) -> Layout {
        Self::BYTES
            .checked_add(len)
            .and_then(|size| Layout::from_size_align(size, align_of::<Inner<H>>()).ok())
            .expect("the string fits in memory")
    }

    /// Allocates `len` uninitialized string bytes and initializes the header with strong count 1.
    fn allocate(header: H, len: usize) -> Self {
        let layout = Self::layout(len);

        let raw = unsafe { alloc::alloc(layout) }.cast::<Inner<H>>();
        let Some(inner) = NonNull::new(raw) else {
            alloc::handle_alloc_error(layout);
        };

        unsafe {
            inner.as_ptr().write(Inner {
                strong: Cell::new(1),
                len,
                header,
            });
        }

        Self {
            inner,
            marker: PhantomData,
        }
    }

    /// Computes the first string-byte pointer from the allocation pointer, allowing it to point beyond `Inner`.
    fn bytes(&self) -> *mut u8 {
        unsafe { self.inner.as_ptr().cast::<u8>().add(Self::BYTES) }
    }

    #[inline]
    fn inner(&self) -> &Inner<H> {
        unsafe { self.inner.as_ref() }
    }

    #[inline]
    #[must_use]
    pub fn header(&self) -> H {
        self.inner().header
    }

    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        let len = self.inner().len;

        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.bytes(), len)) }
    }

    /// Returns whether `this` and `other` share an allocation, like `Rc::ptr_eq`.
    #[inline]
    #[must_use]
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        this.inner == other.inner
    }

    #[must_use]
    pub fn strong_count(this: &Self) -> usize {
        this.inner().strong.get()
    }

    /// Deallocates the string without invoking user code or panicking so `ThinStr::drop` cannot panic.
    #[inline(never)]
    fn deallocate(&mut self) {
        let size = Self::BYTES + self.inner().len;

        unsafe {
            let layout = Layout::from_size_align_unchecked(size, align_of::<Inner<H>>());

            alloc::dealloc(self.inner.as_ptr().cast::<u8>(), layout);
        }
    }
}

impl<H: Copy> Clone for ThinStr<H> {
    #[inline]
    fn clone(&self) -> Self {
        let strong = &self.inner().strong;
        let count = strong.get().wrapping_add(1);

        strong.set(count);

        if count == 0 {
            on_overflow();
        }

        Self {
            inner: self.inner,
            marker: PhantomData,
        }
    }
}

#[cold]
#[inline(never)]
fn on_overflow() -> ! {
    use std::io::Write;

    let _ = writeln!(
        std::io::stderr(),
        "ThinStr reference count overflow; aborting"
    );

    std::process::abort()
}

impl<H: Copy> Drop for ThinStr<H> {
    #[inline(never)]
    fn drop(&mut self) {
        let strong = &self.inner().strong;
        let count = strong.get() - 1;

        strong.set(count);
        if count == 0 {
            self.deallocate();
        }
    }
}

impl<H: Copy> Deref for ThinStr<H> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl<H: Copy> AsRef<str> for ThinStr<H> {
    fn as_ref(&self) -> &str {
        self
    }
}

impl<H: Copy> fmt::Debug for ThinStr<H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl<H: Copy> fmt::Display for ThinStr<H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}
