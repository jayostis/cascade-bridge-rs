use crate::error::Error;
use oxigraph::sparql::CancellationToken;
use std::alloc::{GlobalAlloc, Layout};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize, Ordering::Relaxed};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;
// std's clock panics on wasm32-unknown-unknown, where this one asks the host.
use web_time::Instant;

const MEBIBYTE: usize = 1 << 20;

/// How often, in allocations and in steps of the compiled executor, the clock is read.
const PERIOD: u32 = 1 << 13;

/// What a convert may spend before it is stopped. Both budgets are enforced only where
/// [`Counted`] is the global allocator: it counts the heap and reads the clock for the
/// general evaluator, whose own check of a stopped convert never does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budgets {
    /// Bytes the heap may hold: everything live in the process, the adapters loaded in
    /// it and any earlier conversion not yet freed included. Converts running at once
    /// on native threads share one heap, so one's growth stops the others.
    pub memory_bytes: usize,
    pub time: Duration,
}

impl Budgets {
    /// The most a memory budget may be, in MiB: the 4 GiB of a wasm32 heap leaves no
    /// room above it for a heap that passes its budget before it traps.
    pub const MAX_MEMORY_MIB: u64 = 3072;

    /// The bytes of a whole number of MiB from 1 to [`Self::MAX_MEMORY_MIB`].
    pub fn memory_mib(mebibytes: u64) -> Option<usize> {
        if (1..=Self::MAX_MEMORY_MIB).contains(&mebibytes) {
            Some(mebibytes as usize * MEBIBYTE)
        } else {
            None
        }
    }
}

impl Default for Budgets {
    fn default() -> Self {
        Self {
            memory_bytes: 1024 * MEBIBYTE,
            time: Duration::from_secs(120),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Spent {
    Memory = 1,
    Time = 2,
}

struct Shared {
    token: CancellationToken,
    budgets: Budgets,
    deadline: Option<Instant>,
    spent: AtomicU8,
    beat: AtomicU32,
}

impl Shared {
    fn spent(&self) -> Option<Spent> {
        match self.spent.load(Relaxed) {
            1 => Some(Spent::Memory),
            2 => Some(Spent::Time),
            _ => None,
        }
    }

    fn trip(&self, budget: Spent) {
        if self
            .spent
            .compare_exchange(0, budget as u8, Relaxed, Relaxed)
            .is_ok()
        {
            self.token.cancel();
        }
    }

    fn check_memory(&self) {
        if LIVE.load(Relaxed) > self.budgets.memory_bytes {
            self.trip(Spent::Memory);
        }
    }

    fn check_time(&self) {
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.trip(Spent::Time);
        }
    }
}

/// One convert's budgets, and the token its queries are cancelled by.
#[derive(Clone)]
pub(crate) struct Watch(Arc<Shared>);

impl Default for Watch {
    fn default() -> Self {
        Self::new(Budgets::default())
    }
}

impl Watch {
    pub(crate) fn new(budgets: Budgets) -> Self {
        let shared = Arc::new(Shared {
            token: CancellationToken::new(),
            budgets,
            deadline: Instant::now().checked_add(budgets.time),
            spent: AtomicU8::new(0),
            beat: AtomicU32::new(0),
        });
        let mut armed = armed();
        armed.retain(|watch| watch.strong_count() > 0);
        armed.push(Arc::downgrade(&shared));
        lower_ceiling(&armed);
        Self(shared)
    }

    pub(crate) fn token(&self) -> CancellationToken {
        self.0.token.clone()
    }

    /// True once a budget is spent. Compares the heap each time and reads the clock
    /// once in each period.
    pub(crate) fn pulse(&self) -> bool {
        let beat = self.0.beat.load(Relaxed);
        self.0.beat.store(beat.wrapping_add(1), Relaxed);
        self.0.check_memory();
        if beat.is_multiple_of(PERIOD) {
            self.0.check_time();
        }
        self.0.spent.load(Relaxed) != 0
    }

    fn outgrew(&self) -> Option<String> {
        let budgets = self.0.budgets;
        let said = match self.0.spent()? {
            Spent::Memory if budgets.memory_bytes.is_multiple_of(MEBIBYTE) => {
                format!("memory budget of {} MiB", budgets.memory_bytes / MEBIBYTE)
            }
            Spent::Memory => format!("memory budget of {} bytes", budgets.memory_bytes),
            Spent::Time => format!("time budget of {:?}", budgets.time),
        };
        Some(format!("the convert outgrew its {said}"))
    }

    /// The error that ends the query `query` ran in, where a budget is spent.
    pub(crate) fn outgrown(&self, query: &str) -> Option<Error> {
        Some(Error::outgrown(format!(
            "{query} was stopped: {}",
            self.outgrew()?
        )))
    }

    /// The error that ends a convert whose last query has run, where a budget is spent.
    pub(crate) fn outgrown_by_its_end(&self) -> Option<Error> {
        self.0.check_memory();
        self.0.check_time();
        Some(Error::outgrown(self.outgrew()?))
    }
}

static LIVE: AtomicUsize = AtomicUsize::new(0);
static CEILING: AtomicUsize = AtomicUsize::new(usize::MAX);
static ARMED: Mutex<Vec<Weak<Shared>>> = Mutex::new(Vec::new());
static ALLOCATIONS: AtomicU32 = AtomicU32::new(0);
static READING_CLOCK: AtomicBool = AtomicBool::new(false);

fn armed() -> std::sync::MutexGuard<'static, Vec<Weak<Shared>>> {
    ARMED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn watching(armed: &[Weak<Shared>]) -> impl Iterator<Item = Arc<Shared>> + '_ {
    armed.iter().filter_map(Weak::upgrade)
}

fn lower_ceiling(armed: &[Weak<Shared>]) {
    let lowest = watching(armed)
        .filter(|shared| shared.spent().is_none())
        .map(|shared| shared.budgets.memory_bytes)
        .min()
        .unwrap_or(usize::MAX);
    CEILING.store(lowest, Relaxed);
}

#[cold]
fn over_ceiling(live: usize) {
    let Ok(armed) = ARMED.try_lock() else {
        return;
    };
    for shared in watching(&armed).filter(|shared| live > shared.budgets.memory_bytes) {
        shared.trip(Spent::Memory);
    }
    lower_ceiling(&armed);
}

#[cold]
fn reading_clock() {
    if READING_CLOCK.swap(true, Relaxed) {
        return;
    }
    if let Ok(armed) = ARMED.try_lock() {
        for shared in watching(&armed) {
            shared.check_time();
        }
    }
    READING_CLOCK.store(false, Relaxed);
}

fn allocated(size: usize) {
    let live = LIVE.fetch_add(size, Relaxed).wrapping_add(size);
    if live > CEILING.load(Relaxed) {
        over_ceiling(live);
    }
    let allocations = ALLOCATIONS.load(Relaxed);
    ALLOCATIONS.store(allocations.wrapping_add(1), Relaxed);
    if allocations.is_multiple_of(PERIOD) {
        reading_clock();
    }
}

fn released(size: usize) {
    LIVE.fetch_sub(size, Relaxed);
}

/// A global allocator that counts the bytes it holds and stops the converts whose
/// memory budget that passes.
pub struct Counted<A>(pub A);

unsafe impl<A: GlobalAlloc> GlobalAlloc for Counted<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let allocated_at = self.0.alloc(layout);
        if !allocated_at.is_null() {
            allocated(layout.size());
        }
        allocated_at
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let allocated_at = self.0.alloc_zeroed(layout);
        if !allocated_at.is_null() {
            allocated(layout.size());
        }
        allocated_at
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.dealloc(ptr, layout);
        released(layout.size());
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let moved_to = self.0.realloc(ptr, layout, new_size);
        if !moved_to.is_null() {
            released(layout.size());
            allocated(new_size);
        }
        moved_to
    }
}

#[cfg(test)]
mod tests;
