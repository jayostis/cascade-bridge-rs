#[cfg(target_feature = "atomics")]
compile_error!(
    "the global allocator is shared without a lock, which is sound only while wasm32 runs one thread, so it is built without atomics"
);

use core::alloc::{GlobalAlloc, Layout};
use core::arch::wasm32::memory_size;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, Ordering};
use dlmalloc::Dlmalloc;

const GROWTH_AT_LEAST: usize = 16 * 1024 * 1024;

struct GrowingInLargeSteps {
    dlmalloc: UnsafeCell<Dlmalloc>,
}

unsafe impl Sync for GrowingInLargeSteps {}

#[global_allocator]
static ALLOCATOR: GrowingInLargeSteps = GrowingInLargeSteps {
    dlmalloc: UnsafeCell::new(
        const {
            let mut dlmalloc = Dlmalloc::new();
            assert!(dlmalloc.set_granularity(GROWTH_AT_LEAST));
            dlmalloc
        },
    ),
};

static MEMORY_GROW_CALLS: AtomicU32 = AtomicU32::new(0);

#[no_mangle]
pub extern "C" fn memory_grow_calls() -> u32 {
    MEMORY_GROW_CALLS.load(Ordering::Relaxed)
}

impl GrowingInLargeSteps {
    unsafe fn counting_grows(&self, allocate: impl FnOnce(&mut Dlmalloc) -> *mut u8) -> *mut u8 {
        let pages = memory_size::<0>();
        let allocated = allocate(&mut *self.dlmalloc.get());
        if memory_size::<0>() != pages {
            MEMORY_GROW_CALLS.fetch_add(1, Ordering::Relaxed);
        }
        allocated
    }
}

unsafe impl GlobalAlloc for GrowingInLargeSteps {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.counting_grows(|dlmalloc| dlmalloc.malloc(layout.size(), layout.align()))
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.counting_grows(|dlmalloc| dlmalloc.calloc(layout.size(), layout.align()))
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        (*self.dlmalloc.get()).free(ptr, layout.size(), layout.align())
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.counting_grows(|dlmalloc| {
            dlmalloc.realloc(ptr, layout.size(), layout.align(), new_size)
        })
    }
}
