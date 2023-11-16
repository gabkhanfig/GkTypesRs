use super::allocator::{AllocatorTrait, AllocErr, Allocator};
use std::{alloc::{alloc, dealloc, Layout}, mem::MaybeUninit, sync::Once};

pub struct HeapAllocator{}

impl AllocatorTrait for HeapAllocator {
    fn new_impl() -> Box<dyn AllocatorTrait>
    where Self: Sized {
        return Box::new(HeapAllocator{});
    }

    fn malloc(&self, layout: Layout) -> Result<*mut u8, super::allocator::AllocErr> {
        unsafe {
            let ptr = alloc(layout);
            if ptr.is_null() {
                return Err(AllocErr::OutOfMemory);
            }

            return Ok(ptr);
        }
        
    }

    fn free(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            dealloc(ptr, layout);
        }
    }
}

pub fn global_heap_allocator() -> &'static Allocator {
    static mut GLOBAL_HEAP_ALLOCATOR: MaybeUninit<Allocator> = MaybeUninit::uninit();//HeapAllocator::new();
    static ONCE: Once = Once::new();
    unsafe {
        ONCE.call_once(|| {
            GLOBAL_HEAP_ALLOCATOR.write(HeapAllocator::new());
        });
        return GLOBAL_HEAP_ALLOCATOR.assume_init_ref();
    }
}