/*
/// Holds the free functions, object member functions, or closures to be dispatched by the job,
/// optionally taking a run buffer in.
/// 
/// Technical detail: Uses 24 bytes out of the 64 bytes in a cache line, leaving 40 remaining for JobRunDataBuffer
enum JobFunc {
    FreeFunction(fn ()),
    FreeFunctionBuffer(fn (JobRunDataBuffer)),
    Member(UnsafeCell<Box<dyn FnMut()>>),
    MemberBuffer(UnsafeCell<Box<dyn FnMut(JobRunDataBuffer)>>),
    Closure(UnsafeCell<Box<dyn FnMut()>>),
    ClosureBuffer(UnsafeCell<Box<dyn FnMut(JobRunDataBuffer)>>),
    Invalid
}

impl JobFunc {
    fn new_from_func(func: fn()) -> Self {
        return JobFunc::FreeFunction(func);
    }

    fn new_from_func_buffer(func: fn (JobRunDataBuffer)) -> Self {
        return JobFunc::FreeFunctionBuffer(func);
    }

    unsafe fn new_from_obj<T>(object: *const T, func: fn (&T)) -> Self 
    where T: 'static {
        assert!(!object.is_null());

        let closure = move || func(&*object);
        return JobFunc::Member(UnsafeCell::new(Box::new(closure)));
    }

    unsafe fn new_from_obj_mut<T>(object: *mut T, func: fn (&mut T)) -> Self 
    where T: 'static {
        assert!(!object.is_null());

        let closure = move || func(&mut *object);
        return JobFunc::Member(UnsafeCell::new(Box::new(closure)));
    }

    unsafe fn new_from_obj_buffer<T>(object: *const T, func: fn (&T, JobRunDataBuffer)) -> Self 
    where T: 'static {
        assert!(!object.is_null());

        let closure = move |job_buffer: JobRunDataBuffer| func(&*object, job_buffer);
        return JobFunc::MemberBuffer(UnsafeCell::new(Box::new(closure)));
    }

    unsafe fn new_from_obj_buffer_mut<T>(object: *mut T, func: fn (&mut T, JobRunDataBuffer)) -> Self 
    where T: 'static {
        assert!(!object.is_null());

        let closure = move |job_buffer: JobRunDataBuffer| func(&mut *object, job_buffer);
        return JobFunc::MemberBuffer(UnsafeCell::new(Box::new(closure)));
    }

    fn new_from_closure<F>(func: F) -> Self
    where F: FnMut() + 'static {
        return JobFunc::Closure(UnsafeCell::new(Box::new(func)));
    }

    fn new_from_closure_buffer<F>(func: F) -> Self 
    where F: FnMut(JobRunDataBuffer) + 'static {
        return JobFunc::ClosureBuffer(UnsafeCell::new(Box::new(func)));
    }
}

/// Buffer to hold data for a job function. Provides the developer with 40 bytes to store anything necessary.
/// 
/// Technical detail: 40 bytes is to use up the 64 byte (cache line) alignment of a job that are unused by the function itself.
/// ```
/// use shared::engine::job::job::JobRunDataBuffer;
/// assert_eq!(std::mem::size_of::<JobRunDataBuffer>(), 40);
/// ```
pub struct JobRunDataBuffer {
    buffer: UnsafeCell<[usize; 5]>
}

impl JobRunDataBuffer {
    /// Constructs a new job data buffer instance, taking ownership of the data.
    /// ```
    /// use shared::engine::job::job::JobRunDataBuffer;
    /// let buf = JobRunDataBuffer::new::<u32>(1);
    /// # let _ = buf.get::<u32>();
    /// ```
    /// Can use complex data types
    /// ```
    /// # use gk_types_rs::job_system::job::JobRunDataBuffer;
    /// let buf = JobRunDataBuffer::new::<String>(String::from("hello world!"));
    /// # let _ = buf.get::<String>();
    /// ```
    /// Will panic if the size of T is greater than 40 bytes, or if the alignment is greater than 8.
    /// 
    /// Size:
    /// ``` should_panic
    /// # use gk_types_rs::job_system::job::JobRunDataBuffer;
    /// #[derive(Default)]
    /// struct LargeThing{ data: [usize; 6] }
    /// // Will panic
    /// let buf = JobRunDataBuffer::new::<LargeThing>(LargeThing::default());
    /// ```
    /// Alignment:
    /// ``` should_panic
    /// # use gk_types_rs::job_system::job::JobRunDataBuffer;
    /// #[derive(Default)]
    /// #[repr(align(16))]
    /// struct AlignedThing{ data: [usize; 8] }
    /// // Will panic
    /// let buf = JobRunDataBuffer::new::<AlignedThing>(AlignedThing::default());
    /// ```
    pub fn new<T: Default>(data: T) -> Self {
        assert!(size_of::<T>() <= size_of::<JobRunDataBuffer>());
        assert!(align_of::<T>() <= align_of::<JobRunDataBuffer>());

        let buffer = JobRunDataBuffer::default();
        unsafe {           
            let copy_from = &data as *const T;
            let copy_to = (*buffer.buffer.get()).as_mut_ptr() as *mut T;
            std::ptr::copy_nonoverlapping(copy_from, copy_to, 1);        
        }
        std::mem::forget(data);
        return buffer;
    }

    /// Gets ownership of the data held within the buffer. Will make this object un-usable. 
    /// Trusts the programmer to get using the correct generic type.
    /// ```
    /// use shared::engine::job::job::JobRunDataBuffer;
    /// let buf = JobRunDataBuffer::new::<u32>(1);
    /// let num = buf.get::<u32>();
    /// assert_eq!(num, 1);
    /// ```
    /// Just like with JobRunDataBuffer::new::<T>() it works with complex types through memcpy.
    /// ```
    /// # use gk_types_rs::job_system::job::JobRunDataBuffer;
    /// let buf = JobRunDataBuffer::new::<String>(String::from("hello world!"));
    /// let string = buf.get::<String>();
    /// assert_eq!(string, "hello world!".to_string());
    /// ```
    /// Any buffer that has data MUST be gotten. Failure to do so will cause a panic on drop.
    /// ``` should_panic
    /// # use gk_types_rs::job_system::job::JobRunDataBuffer;
    /// let buf = JobRunDataBuffer::new::<u32>(1);
    /// // Don't run get
    /// // let num = buf.get::<u32>();
    /// // Will panic on drop
    /// ```
    /// Will panic if the size of T is greater than 40 bytes, or if the alignment is greater than 8.
    /// 
    /// Size:
    /// ``` should_panic
    /// # use gk_types_rs::job_system::job::JobRunDataBuffer;
    /// # #[derive(Default)]
    /// # struct LargeThing{ data: [usize; 6] }
    /// # let buf = JobRunDataBuffer::new::<u32>(1);
    /// // Will panic
    /// let data = buf.get::<LargeThing>();
    /// ```
    /// Alignment:
    /// ``` should_panic
    /// # use gk_types_rs::job_system::job::JobRunDataBuffer;
    /// # #[derive(Default)]
    /// # #[repr(align(16))]
    /// # struct AlignedThing{ data: [usize; 8] }
    /// # let buf = JobRunDataBuffer::new::<u32>(1);
    /// // Will panic
    /// let data = buf.get::<AlignedThing>();
    /// ```
    pub fn get<T: Default>(self) -> T {
        assert!(size_of::<T>() <= size_of::<JobRunDataBuffer>());
        assert!(align_of::<T>() <= align_of::<JobRunDataBuffer>());
  
        let mut out = T::default();
        unsafe {
            let copy_from = (*self.buffer.get()).as_mut_ptr() as *const T;
            let copy_to = &mut out as *mut T;
            std::ptr::copy_nonoverlapping(copy_from, copy_to, 1);
        }
        std::mem::forget(self);
        return out;
    }

    // Checks that the entire buffer is empty (not holding any data)
    fn is_zeroed(&self) -> bool {
        let buf = unsafe { *self.buffer.get() };
        return buf[0] == 0
            && buf[1] == 0
            && buf[2] == 0
            && buf[3] == 0
            && buf[4] == 0;
    }

}

impl Default for JobRunDataBuffer {
    fn default() -> Self {
        JobRunDataBuffer { buffer: UnsafeCell::new([0; 5]) }
    }
}

impl Drop for JobRunDataBuffer {
    fn drop(&mut self) {
        assert!(self.is_zeroed(), "Job run data buffer was not properly consumed in a job function that expects consumption");
    }
}

/// Container to execute a job with some optional data.
/// All job functions and closures may not return any values
/// 
/// Technical detail: 64 byte size and alignment for cache line.
/// ```
/// # use std::mem::{size_of, align_of};
/// use shared::engine::job::job::Job;
/// assert_eq!(size_of::<Job>(), 64);
/// assert_eq!(align_of::<Job>(), 64);
/// ```
#[repr(align(64))]
pub struct Job {
    func: UnsafeCell<JobFunc>,
    buffer: UnsafeCell<JobRunDataBuffer>
}

impl Job {
    /// Makes a new job from a free function with no arguments and no return value.
    /// ```
    /// # use gk_types_rs::job_system::job::Job;
    /// fn some_function() { /* Do some stuff */ }
    /// let job = Job::from_func(some_function);
    /// # job.invoke();
    /// ```
    pub fn from_func(func: fn()) -> Self {
        return Job { 
            func: UnsafeCell::new(JobFunc::new_from_func(func)), 
            buffer: UnsafeCell::new(JobRunDataBuffer::default()) 
        }
    }

    /// Makes a new job from a free function taking an owned JobRunDataBuffer as an argument, with no return value.
    /// ```
    /// # use gk_types_rs::job_system::job::{Job, JobRunDataBuffer};
    /// fn some_function(buf: JobRunDataBuffer) {
    ///    let mut v = buf.get::<Option<*mut u32>>();
    ///    unsafe { (*v.unwrap()) += 1 }
    /// }
    /// let mut num = 1u32;
    /// let buffer = JobRunDataBuffer::new(Option::Some(&mut num as *mut u32));
    /// let job = Job::from_func_buffer(some_function, buffer);
    /// job.invoke();
    /// assert_eq!(num, 2);
    /// ```
    pub fn from_func_buffer(func: fn (JobRunDataBuffer), buffer: JobRunDataBuffer) -> Self {
        return Job { 
            func: UnsafeCell::new(JobFunc::new_from_func_buffer(func)), 
            buffer: UnsafeCell::new(buffer) 
        }
    }

    /// Makes a new job given an object and a member function that takes no arguments and returns no value.
    /// This function is unsafe because it does not validate the lifetime of the object.
    /// ```
    /// # use gk_types_rs::job_system::job::{Job, JobRunDataBuffer};
    /// struct Example { val: u32 }
    /// impl Example {
    ///     fn validate(&self) { assert_eq!(self.val, 10); }
    /// }
    /// 
    /// let e = Example { val: 10 };
    /// let job = unsafe { Job::from_obj(&e, Example::validate) };
    /// job.invoke();
    /// ```
    pub unsafe fn from_obj<T>(object: &T, func: fn (&T)) -> Self 
    where T: 'static  {
        return Job { 
            func: UnsafeCell::new(JobFunc::new_from_obj(object as *const T, func)), 
            buffer: UnsafeCell::new(JobRunDataBuffer::default()) 
        }
    }

    /// Makes a new job given a mutable object and a member function that takes no arguments and returns no value.
    /// This function is unsafe because it does not validate the lifetime of the object.
    /// ```
    /// # use gk_types_rs::job_system::job::{Job, JobRunDataBuffer};
    /// struct Example { val: u32 }
    /// impl Example {
    ///     fn increment_and_validate(&mut self) { 
    ///         self.val += 1;
    ///         assert_eq!(self.val, 11); 
    ///     }
    /// }
    /// 
    /// let mut e = Example { val: 10 };
    /// let job = unsafe { Job::from_obj_mut(&mut e, Example::increment_and_validate) };
    /// job.invoke();
    /// ```
    pub unsafe fn from_obj_mut<T>(object: &mut T, func: fn (&mut T)) -> Self 
    where T: 'static  {
        return Job { 
            func: UnsafeCell::new(JobFunc::new_from_obj_mut(object as *mut T, func)), 
            buffer: UnsafeCell::new(JobRunDataBuffer::default()) 
        }
    }

    /// Makes a new job given an object and a member function that takes an owned JobRunDataBuffer, and returns no value.
    /// This function is unsafe because it does not validate the lifetime of the object.
    /// ```
    /// # use gk_types_rs::job_system::job::{Job, JobRunDataBuffer};
    /// struct Example { val: u32 }
    /// impl Example {
    ///     fn add_and_validate(&self, buf: JobRunDataBuffer) { 
    ///         assert_eq!(self.val + buf.get::<u32>(), 12); 
    ///     }
    /// }
    /// 
    /// let e = Example { val: 10 };
    /// let job = unsafe { Job::from_obj_buffer(&e, Example::add_and_validate, JobRunDataBuffer::new::<u32>(2)) };
    /// job.invoke();
    /// ```
    pub unsafe fn from_obj_buffer<T>(object: &T, func: fn (&T, JobRunDataBuffer), buffer: JobRunDataBuffer) -> Self 
    where T: 'static  {
        return Job { 
            func: UnsafeCell::new(JobFunc::new_from_obj_buffer(object as *const T, func)), 
            buffer: UnsafeCell::new(buffer) 
        }
    }

    /// Makes a new job given a mutable object and a member function that takes an owned JobRunDataBuffer, and returns no value.
    /// This function is unsafe because it does not validate the lifetime of the object.
    /// ```
    /// # use gk_types_rs::job_system::job::{Job, JobRunDataBuffer};
    /// struct Example { val: u32 }
    /// impl Example {
    ///     fn mut_add_and_validate(&mut self, buf: JobRunDataBuffer) { 
    ///         self.val += buf.get::<u32>();
    ///         assert_eq!(self.val, 12); 
    ///     }
    /// }
    /// 
    /// let mut e = Example { val: 10 };
    /// let job = unsafe { Job::from_obj_buffer_mut(&mut e, Example::mut_add_and_validate, JobRunDataBuffer::new::<u32>(2)) };
    /// job.invoke();
    /// ```
    pub unsafe fn from_obj_buffer_mut<T>(object: &mut T, func: fn (&mut T, JobRunDataBuffer), buffer: JobRunDataBuffer) -> Self 
    where T: 'static {
        return Job { 
            func: UnsafeCell::new(JobFunc::new_from_obj_buffer_mut(object as *mut T, func)), 
            buffer: UnsafeCell::new(buffer) 
        }
    }

    /// Makes a new job given a closure that does not take a JobRunDataBuffer, and returns no value.
    /// ```
    /// # use gk_types_rs::job_system::job::{Job, JobRunDataBuffer};
    /// let mut v = vec![1, 2, 3, 4, 5];
    /// let job = Job::from_closure(move || {
    ///     v.push(6);
    ///     assert_eq!(v.len(), 6);
    /// });
    /// job.invoke();
    /// ```
    pub fn from_closure<F>(func: F) -> Self
    where F: FnMut() + 'static {
        return Job { 
            func: UnsafeCell::new(JobFunc::new_from_closure(func)), 
            buffer: UnsafeCell::new(JobRunDataBuffer::default()) 
        }
    }

    /// Makes a new job given a closure that takes a JobRunDataBuffer, and returns no value.
    /// While this likely won't be useful, it is present for consistency.
    /// ```
    /// # use gk_types_rs::job_system::job::{Job, JobRunDataBuffer};
    /// let mut v = vec![1, 2, 3, 4, 5];
    /// let job = Job::from_closure_buffer(move |buffer: JobRunDataBuffer| {
    ///     let mut v_closure = buffer.get::<Vec<u32>>();
    ///     v_closure.push(6);
    ///     assert_eq!(v_closure.len(), 6);
    /// }, JobRunDataBuffer::new::<Vec<u32>>(v));
    /// job.invoke();
    /// ```
    pub fn from_closure_buffer<F>(func: F, buffer: JobRunDataBuffer) -> Self 
    where F: FnMut(JobRunDataBuffer) + 'static {
        return Job { 
            func: UnsafeCell::new(JobFunc::new_from_closure_buffer(func)), 
            buffer: UnsafeCell::new(buffer) 
        }
    }

    /// Invokes a job and then invalidates it. It cannot be run more than once.
    /// ```
    /// # use gk_types_rs::job_system::job::Job;
    /// fn some_function() { /* Do some stuff */ }
    /// let job = Job::from_func(some_function);
    /// job.invoke();
    /// ```
    /// Trying to invoke the job a second time will cause a panic.
    /// ``` should_panic
    /// # use gk_types_rs::job_system::job::Job;
    /// # fn some_function() { /* Do some stuff */ }
    /// let job = Job::from_func(some_function);
    /// job.invoke();
    /// // Will panic
    /// job.invoke();
    /// ```
    pub fn invoke(&self) {
        // Transfer buffer ownership into the job function
        let owned_buffer = std::mem::replace(unsafe { &mut *self.buffer.get() }, JobRunDataBuffer::default());
        // Transfer ownership of job function
        let func_to_execute = std::mem::replace(unsafe{ &mut *self.func.get() }, JobFunc::Invalid);

        match func_to_execute {
            JobFunc::FreeFunction(execute) => execute(),
            JobFunc::FreeFunctionBuffer(execute) => execute(owned_buffer),
            JobFunc::Member(execute) => (unsafe { &mut *execute.get() }).as_mut()(),
            JobFunc::MemberBuffer(execute) => (unsafe { &mut *execute.get() }).as_mut()(owned_buffer),
            JobFunc::Closure(execute) => (unsafe { &mut *execute.get() }).as_mut()(),
            JobFunc::ClosureBuffer(execute) => (unsafe { &mut *execute.get() }).as_mut()(owned_buffer),
            JobFunc::Invalid => panic!("Cannot invoke an invalid job")
        }
    }

    pub fn is_bound(&self) -> bool {
        match unsafe { &*self.func.get() } {
            JobFunc::Invalid => false,
            _ => true
        }
    }
}

impl Default for Job {
    fn default() -> Self {
        Self { func: UnsafeCell::new(JobFunc::Invalid), buffer: UnsafeCell::new(JobRunDataBuffer::default()) }
    }
} */


pub(crate) struct JobContainer {
    func: Option<Box<dyn FnMut()>>
}

impl JobContainer {
    pub(crate) fn new<F>(func: F) -> Self
    where F: FnMut() + 'static {
        return JobContainer { func: Some(Box::new(func)) }
    }

    /// Cannot invoke again
    pub(crate) fn invoke(&mut self) {
        let mut f = self.func.take().expect("Cannot invoke None Job func");
        f();
    }
}

impl Default for JobContainer {
    fn default() -> Self {
        Self { func: None }
    }
}