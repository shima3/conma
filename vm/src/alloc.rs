//! The global allocator. The VM makes and drops small Rc blocks (LVNodes,
//! Closures, CFrames, LList cells, Strings) at every step, and the allocator of
//! macOS spends about half of the time of a run on them (it also clears every
//! block it frees). Blocks of up to 256 bytes are therefore kept in free lists,
//! one for each multiple of 16 bytes, and carved from chunks of 256 KB; larger
//! blocks go to the system allocator. Freed small blocks are reused for blocks
//! of the same size and never given back to the system.
//!
//! The VM runs in one thread, but the main thread allocates too (when it
//! starts the VM thread), so the free lists are behind a spin lock, which is
//! never contended in practice.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};

const GRAIN: usize = 16;
const CLASSES: usize = 16; // blocks of 16, 32, ..., 256 bytes
const MAX_SMALL: usize = GRAIN * CLASSES;
const CHUNK: usize = 256 * 1024;

struct Lists {
    free: [*mut u8; CLASSES],
    bump: *mut u8,
    end: *mut u8,
}

pub struct Pool {
    lock: AtomicBool,
    lists: UnsafeCell<Lists>,
}

// The lists are only used while the lock is held.
unsafe impl Sync for Pool {}

impl Pool {
    pub const fn new() -> Pool {
        Pool {
            lock: AtomicBool::new(false),
            lists: UnsafeCell::new(Lists {
                free: [std::ptr::null_mut(); CLASSES],
                bump: std::ptr::null_mut(),
                end: std::ptr::null_mut(),
            }),
        }
    }

    fn with<R>(&self, f: impl FnOnce(&mut Lists) -> R) -> R {
        while self.lock.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            std::hint::spin_loop();
        }
        let r = f(unsafe { &mut *self.lists.get() });
        self.lock.store(false, Ordering::Release);
        r
    }
}

/// The class of a small block, or None for a block that goes to the system.
fn class(layout: &Layout) -> Option<usize> {
    if layout.size() <= MAX_SMALL && layout.align() <= GRAIN {
        Some((layout.size().max(1) - 1) / GRAIN)
    } else {
        None
    }
}

unsafe impl GlobalAlloc for Pool {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let Some(c) = class(&layout) else { return unsafe { System.alloc(layout) } };
        let size = (c + 1) * GRAIN;
        self.with(|l| unsafe {
            let p = l.free[c];
            if !p.is_null() {
                l.free[c] = *(p as *mut *mut u8);
                return p;
            }
            if (l.end as usize) - (l.bump as usize) < size {
                // The rest of the old chunk is left unused.
                let chunk = System.alloc(Layout::from_size_align_unchecked(CHUNK, GRAIN));
                if chunk.is_null() {
                    return chunk;
                }
                l.bump = chunk;
                l.end = chunk.add(CHUNK);
            }
            let p = l.bump;
            l.bump = p.add(size);
            p
        })
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let Some(c) = class(&layout) else { return unsafe { System.dealloc(ptr, layout) } };
        self.with(|l| unsafe {
            *(ptr as *mut *mut u8) = l.free[c];
            l.free[c] = ptr;
        })
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        match (class(&layout), class(&new_layout)) {
            (None, None) => unsafe { System.realloc(ptr, layout, new_size) },
            (Some(a), Some(b)) if a == b => ptr,
            _ => unsafe {
                let p = self.alloc(new_layout);
                if !p.is_null() {
                    std::ptr::copy_nonoverlapping(ptr, p, layout.size().min(new_size));
                    self.dealloc(ptr, layout);
                }
                p
            },
        }
    }
}
