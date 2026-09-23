//! The C allocation calls the vendored core makes, on wasm32 where there is
//! no C library: each block keeps its size in front, for `free` and
//! `realloc`. `memcpy` and `memset` come from Rust's compiler builtins.

use std::alloc::{alloc, alloc_zeroed, dealloc, Layout};
use std::ffi::c_void;

/// Room for the size in front, keeping the block's alignment.
const HEADER: usize = 16;

fn layout(size: usize) -> Option<Layout> {
    Layout::from_size_align(size.checked_add(HEADER)?, HEADER).ok()
}

/// Allocates `size` bytes, zeroed when `zeroed`.
unsafe fn allocate(size: usize, zeroed: bool) -> *mut c_void {
    let Some(layout) = layout(size) else {
        return std::ptr::null_mut();
    };
    // SAFETY: the layout is non-zero-sized, as it includes the header.
    let block = unsafe {
        if zeroed {
            alloc_zeroed(layout)
        } else {
            alloc(layout)
        }
    };
    if block.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: the block holds at least HEADER bytes, aligned for usize.
    unsafe {
        block.cast::<usize>().write(size);
        block.add(HEADER).cast()
    }
}

/// C `malloc`.
///
/// # Safety
/// As C's.
#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    // SAFETY: forwarded contract.
    unsafe { allocate(size, false) }
}

/// C `calloc`.
///
/// # Safety
/// As C's.
#[no_mangle]
pub unsafe extern "C" fn calloc(count: usize, size: usize) -> *mut c_void {
    match count.checked_mul(size) {
        // SAFETY: forwarded contract.
        Some(size) => unsafe { allocate(size, true) },
        None => std::ptr::null_mut(),
    }
}

/// C `free`.
///
/// # Safety
/// `pointer` is null or came from these functions and is not used again.
#[no_mangle]
pub unsafe extern "C" fn free(pointer: *mut c_void) {
    if pointer.is_null() {
        return;
    }
    // SAFETY: the block starts HEADER bytes before, with its size there.
    unsafe {
        let block = pointer.cast::<u8>().sub(HEADER);
        let size = block.cast::<usize>().read();
        if let Some(layout) = layout(size) {
            dealloc(block, layout);
        }
    }
}

/// C `realloc`.
///
/// # Safety
/// As C's.
#[no_mangle]
pub unsafe extern "C" fn realloc(pointer: *mut c_void, size: usize) -> *mut c_void {
    // SAFETY: forwarded contract; the old block's size is in its header.
    unsafe {
        let moved = allocate(size, false);
        if !pointer.is_null() && !moved.is_null() {
            let old = pointer.cast::<u8>().sub(HEADER).cast::<usize>().read();
            std::ptr::copy_nonoverlapping(pointer.cast::<u8>(), moved.cast(), old.min(size));
            free(pointer);
        }
        moved
    }
}
