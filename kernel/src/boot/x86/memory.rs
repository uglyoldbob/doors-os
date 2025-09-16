//! Memory management code that is common to both 32 and 64 bit x86.
//! The heap is a linked list of memory areas ([HeapNode]).
//! Memory is organized by blocks that represent free memory.

use core::{alloc::Layout, ptr::NonNull};

use alloc::alloc::AllocError;

use crate::{Locked, LockedArc};

use super::boot::memory::{Page, PagingTableManager};

pub use super::boot::memory::BumpAllocator;
