//! Opt-in diagnostic counters. No formatting, allocation, or locks in measured scopes.
//! Rust allocator calls only: native plugin allocations require separate profiling.
#[derive(Clone, Copy)]
pub enum Role {
    Callback,
    Feeder,
    Worker,
}

#[cfg(not(feature = "rt-audit"))]
pub struct Scope;
#[cfg(not(feature = "rt-audit"))]
impl Scope {
    #[inline]
    pub fn enter(_: Role) -> Self {
        Self
    }
}

#[cfg(feature = "rt-audit")]
pub use enabled::*;
#[cfg(feature = "rt-audit")]
mod enabled {
    use super::Role;
    use std::{
        alloc::{GlobalAlloc, Layout, System},
        cell::Cell,
        sync::atomic::{AtomicU64, Ordering::Relaxed},
        time::Instant,
    };
    thread_local! { static ROLE: Cell<Option<usize>> = const { Cell::new(None) }; }
    struct Counters {
        allocations: AtomicU64,
        frees: AtomicU64,
        calls: AtomicU64,
        max_ns: AtomicU64,
    }
    static COUNTERS: [Counters; 3] = [const {
        Counters {
            allocations: AtomicU64::new(0),
            frees: AtomicU64::new(0),
            calls: AtomicU64::new(0),
            max_ns: AtomicU64::new(0),
        }
    }; 3];
    pub struct Allocator;
    fn count(free: bool) {
        let _ = ROLE.try_with(|r| {
            if let Some(i) = r.get() {
                let c = &COUNTERS[i];
                if free {
                    c.frees.fetch_add(1, Relaxed);
                } else {
                    c.allocations.fetch_add(1, Relaxed);
                }
            }
        });
    }
    unsafe impl GlobalAlloc for Allocator {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            count(false);
            unsafe { System.alloc(l) }
        }
        unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
            count(false);
            unsafe { System.alloc_zeroed(l) }
        }
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
            count(true);
            unsafe { System.dealloc(p, l) }
        }
        unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
            count(false);
            count(true);
            unsafe { System.realloc(p, l, n) }
        }
    }
    pub struct Scope {
        previous: Option<usize>,
        role: usize,
        at: Instant,
    }
    impl Scope {
        pub fn enter(role: Role) -> Self {
            let role = role as usize;
            Self {
                previous: ROLE.with(|r| r.replace(Some(role))),
                role,
                at: Instant::now(),
            }
        }
    }
    impl Drop for Scope {
        fn drop(&mut self) {
            ROLE.with(|r| r.set(self.previous));
            let c = &COUNTERS[self.role];
            c.calls.fetch_add(1, Relaxed);
            c.max_ns.fetch_max(
                self.at.elapsed().as_nanos().min(u64::MAX as u128) as u64,
                Relaxed,
            );
        }
    }
    #[derive(Debug)]
    pub struct Snapshot {
        pub allocations: u64,
        pub frees: u64,
        pub calls: u64,
        pub max_ns: u64,
    }
    /// Call from the test/control thread between phases; never print from an audio scope.
    pub fn take(role: Role) -> Snapshot {
        let c = &COUNTERS[role as usize];
        Snapshot {
            allocations: c.allocations.swap(0, Relaxed),
            frees: c.frees.swap(0, Relaxed),
            calls: c.calls.swap(0, Relaxed),
            max_ns: c.max_ns.swap(0, Relaxed),
        }
    }
}
