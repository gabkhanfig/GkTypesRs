use core::panic;
use std::{mem::{size_of, ManuallyDrop, align_of, MaybeUninit}, marker::PhantomData, ops::{Index, IndexMut}, sync::Once};
use crate::{allocator::heap_allocator::global_heap_allocator, cpu_features::{is_avx512_supported, is_avx2_supported}};
use super::super::allocator::allocator::Allocator;

// is size of pointer + usize
const SMALL_REP_BUFFER_BYTE_CAPACITY: usize = size_of::<usize>() + size_of::<usize>();

const fn can_type_be_small<T>() -> bool {
    return size_of::<T>() <= SMALL_REP_BUFFER_BYTE_CAPACITY && align_of::<T>() <= align_of::<usize>();
}

const fn small_buffer_type_capacity<T>() -> usize {
    if !can_type_be_small::<T>() {
        return 0;
    }
    return SMALL_REP_BUFFER_BYTE_CAPACITY / size_of::<T>();
}



const ARRAY_LIST_LENGTH_BITMASK: usize = isize::MAX as usize;
const ARRAY_LIST_LENGTH_FLAG_BIT: usize = !ARRAY_LIST_LENGTH_BITMASK;

struct ArrayListLength<T> {
    value: usize,
    marker: PhantomData<T>
}

impl<T> ArrayListLength<T> {
    fn new() -> Self {
        ArrayListLength { value: 0, marker: PhantomData }
    }

    #[inline(always)]
    fn len(&self) -> usize {
        if can_type_be_small::<T>() {
            return self.value & ARRAY_LIST_LENGTH_BITMASK;
        }
        else {
            return self.value;
        }
    }

    #[inline(always)]
    fn set_len(&mut self, new_length: usize) {
        if can_type_be_small::<T>() {
            self.value = new_length | (self.value & ARRAY_LIST_LENGTH_FLAG_BIT);
        }
        else {
            self.value = new_length;
        }
    }

    #[inline(always)]
    fn is_small_rep(&self) -> bool {
        if can_type_be_small::<T>() {
            return (self.value & ARRAY_LIST_LENGTH_FLAG_BIT) == 0;
        }
        else {
            return false;
        }
    }

    #[inline(always)]
    fn set_heap_flag(&mut self, flag: bool) {
        if can_type_be_small::<T>() {
            match flag {
                true => self.value = self.value | ARRAY_LIST_LENGTH_FLAG_BIT,
                false => self.value = self.value & ARRAY_LIST_LENGTH_BITMASK
            }
        }
    }
}

struct HeapRep<T> {
    data: *mut T,
    capacity: usize
}

impl<T> HeapRep<T> {
    fn new() -> Self {
        HeapRep { data: std::ptr::null_mut(), capacity: 0 }
    }
}

struct SmallRep {
    buffer: [u8; SMALL_REP_BUFFER_BYTE_CAPACITY]
}

impl SmallRep {
    fn new() -> Self {
        SmallRep { buffer: [0; SMALL_REP_BUFFER_BYTE_CAPACITY] }
    }
}

pub union ArrayListRep<T> {
    small: ManuallyDrop<SmallRep>,
    heap: ManuallyDrop<HeapRep<T>>
}

impl<T> ArrayListRep<T> {
    fn new() -> Self {
        if can_type_be_small::<T>() {
            return ArrayListRep {
                small: ManuallyDrop::new(SmallRep::new())
            }
        }
        else {
            return ArrayListRep {
                heap: ManuallyDrop::new(HeapRep::new())
            }
        }
    }

    fn heap_set_ptr(&mut self, ptr: *mut T) {
        unsafe { 
            let write_location = (&mut self.heap.data) as *mut *mut T;
            std::ptr::write(write_location, ptr); 
        }
    }

    fn heap_set_capacity(&mut self, new_capacity: usize) {
        unsafe { 
            let write_location = (&mut self.heap.capacity) as *mut usize;
            std::ptr::write(write_location, new_capacity); 
        }
    }

    fn small_buffer(&self) -> *const T {
        unsafe {
            return self.small.buffer.as_ptr() as *const T;
        }
    }

    fn small_buffer_mut(&mut self) -> *mut T {
        unsafe {
            return self.small.buffer.as_ptr() as *mut T;
        }
    }

    fn heap_buffer(&self) -> *const T {
        unsafe {
            return self.heap.data;
        }
    }

    fn heap_buffer_mut(&mut self) -> *mut T {
        unsafe {
            return self.heap.data;
        }
    }
}

pub struct ArrayList<T> {
    allocator: Allocator,
    length: ArrayListLength<T>,
    rep: ArrayListRep<T>
}

impl<T> ArrayList<T> {
    /// Creates a new ArrayList with a clone of the passed in allocator.
    /// `global_heap_allocator()` is a sensible default for an allocator.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// # use std::mem::size_of;
    /// let array_list: ArrayList<u32> = ArrayList::new(global_heap_allocator());
    /// assert_eq!(array_list.len(), 0);
    /// // Has a small in-place buffer that is 16 bytes.
    /// assert_eq!(array_list.capacity(), 16 / size_of::<u32>());
    /// ```
    /// Will not have an in-place buffer for types that are either too large, or do not fit alignment rules.
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let array_list: ArrayList<String> = ArrayList::new(global_heap_allocator());
    /// assert_eq!(array_list.len(), 0);
    /// // Does not have an in-place buffer, and does not initially allocate.
    /// assert_eq!(array_list.capacity(), 0);
    /// ```
    pub fn new(allocator: &Allocator) -> Self {
        return ArrayList { 
                allocator: allocator.clone(),
                length: ArrayListLength::new(), 
                rep: ArrayListRep::new(),       
        }
    }

    /// Creates a new ArrayList with a in-place or allocated buffer capacity of AT LEAST capacity param.
    /// Also clones the passed in allocator.
    /// `global_heap_allocator()` is a sensible default for an allocator.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let array_list: ArrayList<u32> = ArrayList::with_capacity(global_heap_allocator(), 10);
    /// assert!(array_list.capacity() >= 10);
    /// ```
    /// For types that fit the requirements to use the in-place buffer, any with_capacity param value less than
    /// the T capacity of the in-place buffer will be discarded.
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// // Does not perform malloc because it can store 4 u32's in-place (64 bit architecture).
    /// let array_list: ArrayList<u32> = ArrayList::with_capacity(global_heap_allocator(), 3);
    /// assert_eq!(array_list.capacity(), 4);
    /// ```
    pub fn with_capacity(allocator: &Allocator, mut capacity: usize) -> Self {
        if capacity == 0 {
            return ArrayList::new(allocator);
        }

        if can_type_be_small::<T>() {
            if capacity <= small_buffer_type_capacity::<T>() {
                return ArrayList::new(allocator);
            }         
        }
        let mut array_list = ArrayList::new(allocator);
        array_list.rep.heap_set_ptr(Self::malloc_heap_buffer(&array_list.allocator, &mut capacity));
        //array_list.rep.heap_set_ptr(array_list.allocator.malloc_buffer(capacity).unwrap());
        array_list.rep.heap_set_capacity(capacity);
        array_list.length.set_heap_flag(true);
        return array_list;
    }

    /* 
    pub unsafe fn from_raw_parts(ptr: *mut T, length: usize, capacity: usize, allocator: &'a Allocator) -> Self {
        todo!()
    }

    pub unsafe fn into_raw_parts(&self) -> (*mut T, usize, usize, &'a Allocator) {
        todo!()
    }*/

    fn is_small_rep(&self) -> bool { return self.length.is_small_rep(); }

    /// Number of elements stored in the ArrayList
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<u32> = ArrayList::new(global_heap_allocator());
    /// assert_eq!(array_list.len(), 0);
    /// array_list.push(13);
    /// assert_eq!(array_list.len(), 1);
    /// ```
    pub fn len(&self) -> usize { return self.length.len(); }

    /// Number of elements that the ArrayList has already allocated for.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let array_list: ArrayList<String> = ArrayList::new(global_heap_allocator());
    /// assert_eq!(array_list.capacity(), 0);
    /// ```
    /// For types that quality for using the in-place buffer, 
    /// returns the amount that can be held OR the heap allocation capacity.
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// // No capacity reserved
    /// let array_list1: ArrayList<u32> = ArrayList::new(global_heap_allocator());
    /// assert_eq!(array_list1.capacity(), 4);
    /// // Some capacity reserved
    /// let array_list2: ArrayList<u32> = ArrayList::with_capacity(global_heap_allocator(), 25);
    /// assert!(array_list2.capacity() >= 25);
    /// ```
    pub fn capacity(&self) -> usize {
        if can_type_be_small::<T>() {
            if self.is_small_rep() {
                return small_buffer_type_capacity::<T>();
            }
        }
        unsafe { return self.rep.heap.capacity }      
    }

    /// Moves an element onto the end of the buffer, re-allocating when necessary.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<u32> = ArrayList::new(global_heap_allocator());
    /// for i in 0..10 {
    ///     array_list.push(i);
    /// }
    /// for i in 0..10 {
    ///     assert_eq!(array_list[i as usize], i);
    /// }
    /// ```
    /// Naturally works with complex types
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<String> = ArrayList::new(global_heap_allocator());
    /// array_list.push(String::from("hello world!"));
    /// assert_eq!(array_list[0], String::from("hello world!"));
    /// ```
    pub fn push(&mut self, element: T) {
        let current_length = self.len();
        let current_capacity = self.capacity();
        if current_length == current_capacity || current_capacity == 0 {
            let min_capacity = (3* (current_capacity + 1)) >> 1; // ~1.5x
            self.reallocate(min_capacity);
        }

        let buffer = self.as_mut_ptr();
        unsafe { std::ptr::write(buffer.offset(current_length as isize), element) };
        self.length.set_len(current_length + 1);
    }

    /// Get a const pointer to the beginning of the array buffer. It may be null if the ArrayList is empty.
    /// 
    /// It is the responsibility of the programmer to ensure that this pointer is valid on use,
    /// because the array could have elements pushed, removed, or whatever else.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<u32> = ArrayList::new(global_heap_allocator());
    /// array_list.push(1);
    /// array_list.push(2);
    /// let buffer = array_list.as_ptr();
    /// unsafe {
    ///     let second_elem = &*buffer.offset(1);
    ///     assert_eq!(second_elem, &2);
    /// }
    /// ```
    pub fn as_ptr(&self) -> *const T {
        unsafe {
            if can_type_be_small::<T>() {
                if self.is_small_rep() {
                    return self.rep.small_buffer();
                }
            }
            return self.rep.heap.data;
        }
    }

    /// Get a mutable pointer to the beginning of the array buffer. It may be null if the ArrayList is empty.
    /// 
    /// It is the responsibility of the programmer to ensure that this pointer is valid on use,
    /// because the array could have elements pushed, removed, or whatever else.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<u32> = ArrayList::new(global_heap_allocator());
    /// array_list.push(1);
    /// array_list.push(2);
    /// let buffer = array_list.as_mut_ptr();
    /// unsafe {
    ///     let second_elem = &mut *buffer.offset(1);
    ///     *second_elem = 4;
    /// }
    /// assert_eq!(array_list[0], 1);
    /// assert_eq!(array_list[1], 4);
    /// ``` 
    pub fn as_mut_ptr(&mut self) -> *mut T {
        unsafe {
            if can_type_be_small::<T>() {
                if self.is_small_rep() {
                    return self.rep.small_buffer_mut();
                }
            }      
            return self.rep.heap.data;
        }
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the given `ArrayList<T>`. It may reserve more space to avoid frequent reallocations.
    /// After calling `reserve`, the capacity will be greater than or equal to `self.len() + additional`.
    /// Does nothing if there is already enough capacity either from the allocator, or the in-place buffer if appropriate.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<u32> = ArrayList::new(global_heap_allocator());
    /// array_list.reserve(10);
    /// assert!(array_list.capacity() >= 10);
    /// ```
    /// Adding capacity
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<u32> = ArrayList::new(global_heap_allocator());
    /// array_list.push(1);
    /// array_list.reserve(10);
    /// assert!(array_list.capacity() >= 11);
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        let current_length = self.len();
        let current_capacity = self.capacity();
        if current_length + additional <= current_capacity {
            return;
        }

        let new_capacity = {
            let normal_increase = (3 * (current_capacity + 1)) >> 1; // ~1.5x
            if current_length + additional > normal_increase {
                current_length + additional
            }
            else {
                normal_increase
            }
        };
        self.reallocate(new_capacity);
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the given `ArrayList<T>`. It WILL NOT reserve more space to avoid frequent reallocations,
    /// but may still reserve extra given any available SIMD buffer sizes.
    /// `reserve_exact` should only be used when no reallocation afterwards is ensured by the developer.
    /// Otherwise, `reserve` should be used.
    /// After calling `reserve_exact`, the capacity will be greater than or equal to `self.len() + additional`.
    /// Does nothing if there is already enough capacity either from the allocator, or the in-place buffer if appropriate.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<u32> = ArrayList::new(global_heap_allocator());
    /// array_list.reserve_exact(10);
    /// assert!(array_list.capacity() >= 10);
    /// ```
    /// Adding capacity
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<u32> = ArrayList::new(global_heap_allocator());
    /// array_list.push(1);
    /// array_list.reserve_exact(10);
    /// assert!(array_list.capacity() >= 11);
    /// ```
    pub fn reserve_exact(&mut self, additional: usize) {
        let current_length = self.len();
        let current_capacity = self.capacity();
        if current_length + additional <= current_capacity {
            return;
        }
        let new_capacity = current_length + additional;
        self.reallocate(new_capacity);
    }

    /// Find the index, linearly, of the first occurence of `element` in the ArrayList.
    /// Unlike `find_simd`, works with all types. A return of `None` indicates that `element` does not exist in the ArrayList.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<u32> = ArrayList::new(global_heap_allocator());
    /// for i in 0..100 {
    ///     array_list.push(i);
    /// }
    /// let found_index = array_list.find(&80);
    /// assert_eq!(found_index.unwrap(), 80);
    /// ```
    /// None is returned when it's not present.
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<u32> = ArrayList::new(global_heap_allocator());
    /// for i in 0..100 {
    ///     array_list.push(i);
    /// }
    /// let found_index = array_list.find(&101);
    /// assert!(found_index.is_none());
    /// ```
    /// Using non SIMD types
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<String> = ArrayList::new(global_heap_allocator());
    /// for i in 0..10 {
    ///     array_list.push(i.to_string());
    /// }
    /// let found_index = array_list.find(&String::from("5"));
    /// assert_eq!(found_index.unwrap(), 5);
    /// ```
    pub fn find(&self, element: &T) -> Option<usize> 
    where T: std::cmp::PartialEq {
        let buf = self.as_ptr();
        let num = self.len() as isize;
        for i in 0..num {
            if unsafe { &*buf.offset(i) == element } {
                return Some(i as usize);
            }
        }
        return None;
    }

    /// Find the index, using SIMD, of the first occurence of `element` in the ArrayList where for `T`, `size_of::<T>()` is equal to `1`, `2`, `4`, or `8`.
    /// Works with pointers and references, which are 8 bytes on 64 bit architecture.
    /// A return of `None` indicates that `element`` does not exist in the ArrayList.
    /// 
    /// # Panics
    /// 
    /// In debug, will panic if `size_of::<T>()` is not equal to `1`, `2`, `4`, or `8`.
    /// 
    /// # Note
    /// 
    /// ArrayList heap allocations are always done 64 byte aligned if `size_of::<T>()` is equal to `1`, `2`, `4`, or `8`,
    /// but the in-place buffer is 8 byte aligned. If using the in-place buffer, will do a normal find.
    /// 
    /// # Examples
    /// 
    /// `size_of::<T>() == 1`:
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<u8> = ArrayList::new(global_heap_allocator());
    /// for i in 0..100 {
    ///     array_list.push(i);
    /// }
    /// let found_index = unsafe { array_list.find_simd(&80) };
    /// assert_eq!(found_index.unwrap(), 80);
    /// ```
    /// `size_of::<T>() == 8`:
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<usize> = ArrayList::new(global_heap_allocator());
    /// for i in 0..100 {
    ///     array_list.push(i);
    /// }
    /// let found_index = unsafe { array_list.find_simd(&80) };
    /// assert_eq!(found_index.unwrap(), 80);
    /// ```
    /// As mentioned above, panics when `size_of::<T>()` is not equal to `1`, `2`, `4`, or `8`.
    /// ``` should_panic
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<String> = ArrayList::new(global_heap_allocator());
    /// // Will panic in debug
    /// let found_index = unsafe { array_list.find_simd(&String::from("hello world!"))};
    /// ```
    pub unsafe fn find_simd(&self, element: &T) -> Option<usize> {
        debug_assert!(size_of::<T>() == 1 || size_of::<T>() == 2 || size_of::<T>() == 4 || size_of::<T>() == 8, "\nType cannot be used for ArrayList SIMD find");
        
        let buffer = self.as_ptr();
        let length = self.len();
        let capacity = self.capacity();
        let num_per_simd = 64 / size_of::<T>();
        if capacity >= num_per_simd {
            return Self::do_simd_find(buffer, length, capacity, element);
        }
        else {
            for index in 0..length as isize {
                if unsafe { buffer.offset(index) == element } {
                    return Some(index as usize);
                }
            }
            return None;
        }
    }

    /// Removes an element at a specific index, shifting over the elements after it downwards.
    /// 
    /// Maintains order but not indices. 
    /// 
    /// It is reasonable to think that any ArrayList that's having elements removed frequently, will also
    /// have elements pushed. Therefore, not reallocating is ideal. The programmer can shrink the array naturally
    /// by using `shrink_to_fit()`.
    /// 
    /// # Panics 
    /// 
    /// If index greater than or equal to `len()`
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<String> = ArrayList::new(global_heap_allocator());
    /// for i in 0..3 {
    ///     array_list.push(i.to_string());
    /// }
    /// array_list.remove(1);
    /// assert_eq!(array_list[0], String::from("0"));
    /// assert_eq!(array_list[1], String::from("2"));
    /// ```
    /// WWill panic if index is out of range.
    /// ``` should_panic
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<String> = ArrayList::new(global_heap_allocator());
    /// for i in 0..3 {
    ///     array_list.push(i.to_string());
    /// }
    /// // Will panic because 0 is out of range. Must be less than or equal to array_list.len()
    /// array_list.remove(4);
    /// ```
    pub fn remove(&mut self, index: usize) -> T {
        let length = self.len();
        assert!(index < length);
        let buffer = self.as_mut_ptr();
        
        let temp = unsafe {
            buffer.offset(index as isize).read()
        };
        unsafe {
            for i in index as isize..(length - 1) as isize {
                let move_to = &mut *buffer.offset(i);
                let move_from = &mut *buffer.offset(i + 1);
                std::mem::swap(move_to, move_from);
            }
        }
        self.length.set_len(length - 1);
        return temp;
    }

    /// Insert an element at a specific index, shifting over the elements at and after the index over.
    /// Reallocates if necessary.
    /// 
    /// # Panics
    /// 
    /// If index greater than or equal to `len()`
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<String> = ArrayList::new(global_heap_allocator());
    /// array_list.push(String::from("world"));
    /// array_list.insert(0, String::from("hello"));
    /// assert_eq!(array_list[0], String::from("hello"));
    /// assert_eq!(array_list[1], String::from("world"));
    /// ```
    /// Will panic if index is out of range.
    /// ``` should_panic
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<String> = ArrayList::new(global_heap_allocator());
    /// // Will panic because 0 is out of range. Must be less than or equal to array_list.len()
    /// array_list.insert(0, String::from("hello"));
    /// ```
    pub fn insert(&mut self, index: usize, element: T) {
        let current_length = self.len();
        assert!(index < current_length);
        let current_capacity = self.capacity();
        if current_length == current_capacity || current_capacity == 0 {
            let min_capacity = (3* (current_capacity + 1)) >> 1; // ~1.5x
            self.reallocate(min_capacity);
        }

        let buffer = self.as_mut_ptr();
        unsafe {
            for i in index as isize..current_length as isize {
                let move_to = &mut *buffer.offset(i + 1);
                let move_from = &mut *buffer.offset(i);
                std::mem::swap(move_to, move_from);
            }
            std::ptr::write(buffer.offset(index as isize), element);
        }
        self.length.set_len(current_length + 1);
        return;
    }

    /// Shrinks the capacity of the ArrayList as much as possible while still adhereing to any SIMD specific optimizations.
    /// It will drop down as close as possible to the length, but may still be greater than the length.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<usize> = ArrayList::with_capacity(global_heap_allocator(), 100);
    /// assert!(array_list.capacity() >= 100);
    /// for i in 0..5 {
    ///     array_list.push(i);
    /// }
    /// array_list.shrink_to_fit();
    /// assert!(array_list.capacity() < 100);
    /// ```
    /// Non SIMD type.
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<String> = ArrayList::with_capacity(global_heap_allocator(), 100);
    /// assert!(array_list.capacity() >= 100);
    /// for i in 0..5 {
    ///     array_list.push(i.to_string());
    /// }
    /// array_list.shrink_to_fit();
    /// assert!(array_list.capacity() < 100);
    /// ```
    pub fn shrink_to_fit(&mut self) {
        let can_simd = const { size_of::<T>() == 1 || size_of::<T>() == 2 || size_of::<T>() == 4 || size_of::<T>() == 8 };
        let current_capacity = self.capacity();
        let min_capacity = {
            let length = self.len();
            if can_simd {
                let num_per_simd = 64 / size_of::<T>();
                let remainder = length  % num_per_simd;
                if remainder != 0 {
                    length + (num_per_simd - remainder)
                }
                else {
                    length
                }
                
            }
            else {
                length
            }
        };

        if current_capacity > min_capacity {
            // TODO investigate if this can be optimized further by not rechecking capacity?
            self.reallocate(min_capacity);
        }   
    }

    /// Shrinks the capacity of the ArrayList to a lower bound while still adhereing to any SIMD specific optimizations.
    /// The capacity will remain at least as large as both the `len()` and `min_capacity`. 
    /// If the current capacity is less than `min_capacity`, does nothing.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<usize> = ArrayList::with_capacity(global_heap_allocator(), 100);
    /// assert!(array_list.capacity() >= 100);
    /// for i in 0..5 {
    ///     array_list.push(i);
    /// }
    /// array_list.shrink_to(50);
    /// assert!(array_list.capacity() >= 50 && array_list.capacity() < 100);
    /// ```
    /// Non SIMD type.
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<String> = ArrayList::with_capacity(global_heap_allocator(), 100);
    /// assert!(array_list.capacity() >= 100);
    /// for i in 0..5 {
    ///     array_list.push(i.to_string());
    /// }
    /// array_list.shrink_to(50);
    /// assert!(array_list.capacity() >= 50 && array_list.capacity() < 100);
    /// ```
    pub fn shrink_to(&mut self, min_capacity: usize) {
        let can_simd = const { size_of::<T>() == 1 || size_of::<T>() == 2 || size_of::<T>() == 4 || size_of::<T>() == 8 };
        let current_capacity = self.capacity();
        if current_capacity < min_capacity {
            return;
        }
        let new_min_capacity = {      
            let lower_bound = {
                let length = self.len();
                if min_capacity < length {
                    length
                }
                else {
                    min_capacity
                }
            };
            if can_simd {
                let num_per_simd = 64 / size_of::<T>();
                let remainder = lower_bound  % num_per_simd;
                if remainder != 0 {
                    lower_bound + (num_per_simd - remainder)
                }
                else {
                    lower_bound
                }
                
            }
            else {
                lower_bound
            }
        };

        if current_capacity > new_min_capacity {
            // TODO investigate if this can be optimized further by not rechecking capacity?
            self.reallocate(new_min_capacity);
        }   
    }

    pub fn as_slice(&self) -> &[T] {
        return unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len()) };
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        return unsafe { std::slice::from_raw_parts_mut(self.as_mut_ptr(), self.len()) };
    }

    /// Get a reference to the allocator this ArrayList is using. Can be cloned.
    pub fn allocator(&self) -> &Allocator {
        return &self.allocator;
    }

    pub unsafe fn set_len(&mut self, new_length: usize) {
        self.length.set_len(new_length);
    }

    /// Removes an element at a specific index and returns it. The last element of the ArrayList will take it's place if 
    /// if `index` is not the last element, of the ArrayList in it's place. Order is most likely not going to be maintained.
    /// 
    /// `swap_remove()` is more performant than `remove()` when the ArrayList has more than 1 element after `index`.
    /// 
    /// It is reasonable to think that any ArrayList that's having elements removed frequently, will also
    /// have elements pushed. Therefore, not reallocating is ideal. The programmer can shrink the array naturally
    /// by using `shrink_to_fit()`.
    /// 
    /// # Panics 
    /// 
    /// If `index` greater than or equal to `len()`
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<u32> = ArrayList::new(global_heap_allocator());
    /// array_list.push(10);
    /// array_list.push(11);
    /// array_list.push(12);
    /// assert_eq!(array_list.swap_remove(1), 11);
    /// assert_eq!(array_list[0], 10);
    /// assert_eq!(array_list[1], 12);
    /// ```
    /// `index` must be in range
    /// ``` should_panic
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<u32> = ArrayList::new(global_heap_allocator());
    /// array_list.push(10);
    /// array_list.push(11);
    /// array_list.push(12);
    /// // Will panic because index 3 is out of range
    /// array_list.swap_remove(3);
    /// ```
    pub fn swap_remove(&mut self, index: usize) -> T {
        let length = self.len();
        assert!(index < length);
        let buffer = self.as_mut_ptr();
        let is_last_element = index == (length - 1);
        
        let temp = unsafe {
            buffer.offset(index as isize).read()
        };
        
        if !is_last_element {
            unsafe {
                let move_to = &mut *buffer.offset(index as isize);
                let move_from = &mut *buffer.offset(length as isize - 1);
                std::mem::swap(move_to, move_from);
                // for i in index as isize..(length - 1) as isize {
                //     let move_to = &mut *buffer.offset(i);
                //     let move_from = &mut *buffer.offset(i + 1);
                //     std::mem::swap(move_to, move_from);
                // }
            }
        }
        self.length.set_len(length - 1);
        return temp;     
    }

    pub fn truncate(&mut self, len: usize) {
        todo!()
    }

    pub fn retain<F>(&mut self, f: F)
        where F: FnMut(&T) -> bool {
        todo!()
    }

    pub fn retain_mut<F>(&mut self, f: F)
        where F: FnMut(&mut T) -> bool {
        todo!()
    }



    fn reallocate(&mut self, mut min_capacity: usize) {
        let current_length = self.len() as isize;
        let new_data: *mut T = Self::malloc_heap_buffer(&self.allocator, &mut min_capacity);
        if !self.is_small_rep() { // is already heap, will need to move all old elements into new buffer and update union members.
            if self.rep.heap_buffer() != std::ptr::null() {               
                for i in 0..current_length {
                    let new_swap_location = unsafe { &mut *new_data.offset(i) };
                    let old_swap_location = unsafe { &mut *self.rep.heap_buffer_mut().offset(i) };
                    std::mem::swap(new_swap_location, old_swap_location);
                }
                Self::free_heap_buffer(&self.allocator, self.rep.heap_buffer_mut(), unsafe { self.rep.heap.capacity });
                self.rep.heap_set_ptr(new_data);
                self.rep.heap_set_capacity(min_capacity);
                return;
            }
        }
        // if it has non zero length, and isn't heap, it is always small buffer.
        for i in 0..current_length {
            let new_swap_location = unsafe { &mut *new_data.offset(i) };
            let old_swap_location = unsafe { &mut *self.rep.small_buffer_mut().offset(i) };
            std::mem::swap(new_swap_location, old_swap_location);
        }
        self.rep.heap_set_ptr(new_data);
        self.rep.heap_set_capacity(min_capacity);
        self.length.set_heap_flag(true);
    }

    /// Will allocate for a buffer on the heap. If the type can be used for SIMD operations, the allocation will be 64 byte aligned, 
    /// and will contain chunks of 64 / size_of::<T>().
    fn malloc_heap_buffer(allocator: &Allocator, capacity: &mut usize) -> *mut T {
        let can_simd = const { size_of::<T>() == 1 || size_of::<T>() == 2 || size_of::<T>() == 4 || size_of::<T>() == 8 };
        if can_simd {
            let num_per_simd = 64 / size_of::<T>();
            let remainder = *capacity % num_per_simd;
            if remainder != 0 {
                *capacity = *capacity + (num_per_simd - remainder);
            }
            return allocator.malloc_aligned_buffer(*capacity, 64).unwrap();
        }
        else {
            return allocator.malloc_buffer(*capacity).unwrap();
        }
    }

    fn free_heap_buffer(allocator: &Allocator, buffer: *mut T, capacity: usize) {
        if size_of::<T>() <= size_of::<usize>() { // can be used for SIMD
            return allocator.free_aligned_buffer(buffer, capacity, 64);
        }
        else {
            return allocator.free_buffer(buffer, capacity);
        }
    }

    fn do_simd_find(buffer: *const T, length: usize, capacity: usize, element: &T) -> Option<usize> {
        static ONCE: Once = Once::new();
        static mut EPI8_FUNC: MaybeUninit<fn (*const i8, usize, usize, i8) -> Option<usize>> = MaybeUninit::uninit(); 
        static mut EPI16_FUNC: MaybeUninit<fn (*const i16, usize, usize, i16) -> Option<usize>> = MaybeUninit::uninit(); 
        static mut EPI32_FUNC: MaybeUninit<fn (*const i32, usize, usize, i32) -> Option<usize>> = MaybeUninit::uninit(); 
        static mut EPI64_FUNC: MaybeUninit<fn (*const i64, usize, usize, i64) -> Option<usize>> = MaybeUninit::uninit(); 
        
        unsafe {
            ONCE.call_once(|| {
                if is_avx512_supported() {
                    EPI8_FUNC.write(crate::array::simd::simd_find_epi8_512);
                    EPI16_FUNC.write(crate::array::simd::simd_find_epi16_512);
                    EPI32_FUNC.write(crate::array::simd::simd_find_epi32_512);
                    EPI64_FUNC.write(crate::array::simd::simd_find_epi64_512);
                }
                else if is_avx2_supported() {
                    EPI8_FUNC.write(crate::array::simd::simd_find_epi8_256);
                    EPI16_FUNC.write(crate::array::simd::simd_find_epi16_256);
                    EPI32_FUNC.write(crate::array::simd::simd_find_epi32_256);
                    EPI64_FUNC.write(crate::array::simd::simd_find_epi64_256);
                }
                else {
                    panic!("AVX-512 and AVX-2 are both not supported");
                }
            });

            match size_of::<T>() {
                1 => {
                    return (*EPI8_FUNC.assume_init_ref())(buffer as *const i8, length, capacity, *(element as *const T as *const i8));
                },
                2 => {
                    return (*EPI16_FUNC.assume_init_ref())(buffer as *const i16, length, capacity, *(element as *const T as *const i16));
                },
                4 => {
                    return (*EPI32_FUNC.assume_init_ref())(buffer as *const i32, length, capacity, *(element as *const T as *const i32));
                },
                8 => {
                    return (*EPI64_FUNC.assume_init_ref())(buffer as *const i64, length, capacity, *(element as *const T as *const i64));
                },
                _ => unreachable!()
            }
        }
    }
}

impl<T> Default for ArrayList<T> {
    fn default() -> Self {
        return ArrayList::new(global_heap_allocator());
    }
}

impl<T> Drop for ArrayList<T> {
    fn drop(&mut self) {
        let length = self.len();
        let ptr = self.as_mut_ptr();

        for i in 0..length as isize {
            unsafe { ptr.offset(i).drop_in_place()};
        }
        if !self.is_small_rep() && !self.rep.heap_buffer_mut().is_null() {
            Self::free_heap_buffer(&self.allocator, self.rep.heap_buffer_mut(), unsafe { self.rep.heap.capacity });
        }
    }
}

impl<T> Index<usize> for ArrayList<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.len());
        unsafe { &*self.as_ptr().offset(index as isize) }
    }
}

impl<T> IndexMut<usize> for ArrayList<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.len());
        unsafe { &mut *self.as_mut_ptr().offset(index as isize) }
    }
}

// https://doc.rust-lang.org/src/alloc/vec/mod.rs.html#2658
// deref