use std::{alloc::Layout, mem::{size_of, align_of}, sync::Arc};

#[derive(Debug)]
pub enum AllocErr {
    OutOfMemory
}

/// Basically a wrapper around Arc<Box<dyn AllocatorTrait>> with helper methods.
/// ```
/// # use gk_types_rs::allocator::allocator::Allocator;
/// # use std::mem::size_of;
/// assert_eq!(size_of::<Allocator>(), 8);
/// ```
#[derive(Clone)]
pub struct Allocator {
    inner: Arc<Box<dyn AllocatorTrait>>
}

impl Allocator {
    pub fn malloc_object<T>(&self) -> Result<*mut T, AllocErr> {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<T>(), align_of::<T>()) };
        let byte_buffer = self.inner.malloc(layout)?;
        return Ok(byte_buffer as *mut T);
    }

    pub fn malloc_object_zero<T>(&self) -> Result<*mut T, AllocErr> {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<T>(), align_of::<T>()) };
        let byte_buffer = self.inner.malloc_zero(layout)?;
        return Ok(byte_buffer as *mut T);
    }

    pub fn malloc_object_default<T: Default>(&self) -> Result<*mut T, AllocErr> {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<T>(), align_of::<T>()) };
        let byte_buffer = self.inner.malloc(layout)?;
        let type_buffer = byte_buffer as *mut T;
        unsafe { std::mem::swap(&mut *type_buffer, &mut T::default()); }
        return Ok(type_buffer);
    }

    pub fn malloc_aligned_object<T>(&self, byte_alignment: usize) -> Result<*mut T, AllocErr> {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<T>(), byte_alignment) };
        let byte_buffer = self.inner.malloc(layout)?;
        return Ok(byte_buffer as *mut T);
    }

    pub fn malloc_aligned_object_zero<T>(&self, byte_alignment: usize) -> Result<*mut T, AllocErr> {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<T>(), byte_alignment) };
        let byte_buffer = self.inner.malloc_zero(layout)?;
        return Ok(byte_buffer as *mut T);
    }

    pub fn malloc_aligned_object_default<T: Default>(&self, byte_alignment: usize) -> Result<*mut T, AllocErr> {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<T>(), byte_alignment) };
        let byte_buffer = self.inner.malloc(layout)?;
        let type_buffer = byte_buffer as *mut T;
        unsafe { std::mem::swap(&mut *type_buffer, &mut T::default()); }
        return Ok(type_buffer);
    }

    pub fn malloc_buffer<T>(&self, num_elements: usize) -> Result<*mut T, AllocErr> {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<T>() * num_elements, align_of::<T>()) };
        let byte_buffer = self.inner.malloc(layout)?;
        return Ok(byte_buffer as *mut T);
    }

    pub fn malloc_buffer_zero<T>(&self, num_elements: usize) -> Result<*mut T, AllocErr> {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<T>() * num_elements, align_of::<T>()) };
        let byte_buffer = self.inner.malloc_zero(layout)?;
        return Ok(byte_buffer as *mut T);  
    }

    pub fn malloc_buffer_default<T: Default>(&self, num_elements: usize) -> Result<*mut T, AllocErr> {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<T>() * num_elements, align_of::<T>()) };
        let byte_buffer = self.inner.malloc(layout)?;
        let type_buffer = byte_buffer as *mut T;
        unsafe { 
            for i in 0..num_elements as isize {
                std::mem::swap(&mut *type_buffer.offset(i), &mut T::default()); 
            }      
        }
        return Ok(type_buffer);
    }

    pub fn malloc_aligned_buffer<T>(&self, num_elements: usize, byte_alignment: usize) -> Result<*mut T, AllocErr> {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<T>() * num_elements, byte_alignment) };
        let byte_buffer = self.inner.malloc(layout)?;
        return Ok(byte_buffer as *mut T);
    }

    pub fn malloc_aligned_buffer_zero<T>(&self, num_elements: usize, byte_alignment: usize) -> Result<*mut T, AllocErr> {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<T>() * num_elements, byte_alignment) };
        let byte_buffer = self.inner.malloc_zero(layout)?;
        return Ok(byte_buffer as *mut T);
    }

    pub fn malloc_aligned_buffer_default<T: Default>(&self, num_elements: usize, byte_alignment: usize) -> Result<*mut T, AllocErr> {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<T>() * num_elements, byte_alignment) };
        let byte_buffer = self.inner.malloc(layout)?;
        let type_buffer = byte_buffer as *mut T;
        unsafe { 
            for i in 0..num_elements as isize {
                std::mem::swap(&mut *type_buffer.offset(i), &mut T::default()); 
            }      
        }
        return Ok(type_buffer);
    }

    pub fn free_object<T>(&self, object: *mut T) {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<T>(), align_of::<T>()) };
        self.inner.free(object as *mut u8, layout);
    }

    pub fn free_object_aligned<T>(&self, object: *mut T, byte_alignment: usize) {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<T>(), byte_alignment) };
        self.inner.free(object as *mut u8, layout);
    }

    pub fn free_buffer<T>(&self, buffer: *mut T, num_elements: usize) {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<T>() * num_elements, align_of::<T>()) };
        self.inner.free(buffer as *mut u8, layout);
    }

    pub fn free_aligned_buffer<T>(&self, buffer: *mut T, num_elements: usize, byte_alignment: usize) {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<T>() * num_elements, byte_alignment) };
        self.inner.free(buffer as *mut u8, layout);
    }

}

unsafe impl Sync for Allocator {}

pub trait AllocatorTrait {

    fn new() -> Allocator
    where Self: Sized {
        return Allocator { inner: Arc::new(Self::new_impl()) }
    }

    fn new_impl() -> Box<dyn AllocatorTrait>
    where Self: Sized;

    fn malloc(&self, layout: Layout) -> Result<*mut u8, super::allocator::AllocErr>;

    fn malloc_zero(&self, layout: Layout) -> Result<*mut u8, super::allocator::AllocErr> {
        unsafe {
            let ptr = self.malloc(layout)?;
            ptr.write_bytes(0, layout.size());
            return Ok(ptr);
        }      
    }

    fn free(&self, ptr: *mut u8, layout: Layout);
}