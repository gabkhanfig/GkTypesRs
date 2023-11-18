use super::array_list::ArrayList;

pub struct ArrayListIter<'a, T> {
    marker: std::marker::PhantomData<&'a T>,
    ptr: *const T,
    index: usize,
    num: usize,
}

impl<'a, T> ArrayListIter<'a, T> {
    pub(crate) fn new(array_list: &'a ArrayList<T>) -> Self {
        let ptr = array_list.as_ptr();
        let num = array_list.len();
        return ArrayListIter { marker: std::marker::PhantomData, ptr, index: 0, num }
    }
}

impl<'a, T> Iterator for ArrayListIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.num {
            return None;
        }
        self.index += 1;
        return Some(unsafe { &*self.ptr.offset((self.index - 1) as isize)});
    }
}

impl<'a, T> IntoIterator for &'a ArrayList<T> {
    type Item = &'a T;
    type IntoIter = ArrayListIter<'a, T>;

    /// Make an iterator from an ArrayList.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::array::array_list::ArrayList;
    /// # use gk_types_rs::allocator::heap_allocator::global_heap_allocator;
    /// let mut array_list: ArrayList<usize> = ArrayList::new(global_heap_allocator());
    /// for i in 1..5 {
    ///     array_list.push(i);
    /// }
    /// for elem in &array_list {
    ///        assert!(elem > &0);
    /// }
    /// ```
    fn into_iter(self) -> Self::IntoIter {
        return ArrayListIter::new(self);
    }
}