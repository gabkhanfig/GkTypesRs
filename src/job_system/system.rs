use std::{sync::atomic::{AtomicUsize, Ordering}, thread, mem::MaybeUninit, cell::UnsafeCell};
use super::{thread::JobThread, future::JobFuture};

pub(crate) const QUEUE_CAPACITY: usize = 8192;

struct Inner {
    threads: Box<[Box<JobThread>]>,
    // MUST not mutate
    thread_count: usize,
    current_optimal_thread: AtomicUsize
}

impl Inner {
    fn get_optimal_thread_for_execution(&mut self) -> usize {
        let previous_optimal = self.current_optimal_thread.load(Ordering::Acquire);
        let mut minimum_queue_load = usize::MAX;
        let mut is_optimal_executing = true;
        let mut current_optimal = previous_optimal;

        for i in 0..self.thread_count {
            let check_index = (previous_optimal + i) % self.thread_count;
            let is_not_executing = !self.threads[check_index].is_executing();
            let queue_load = self.threads[check_index].queued_count();
            if is_not_executing && queue_load == 0 {
                self.current_optimal_thread.store((check_index + 1) % self.thread_count, Ordering::Release);
                return check_index;
            }

            if is_not_executing {
                if minimum_queue_load > queue_load {
                    current_optimal = check_index;
                    minimum_queue_load = queue_load;
                    is_optimal_executing = false;
                    continue;
                }
            }

            if minimum_queue_load > queue_load && is_optimal_executing {
                current_optimal = check_index;
                minimum_queue_load = queue_load;
            }
        }
        self.current_optimal_thread.store((current_optimal + 1) % self.thread_count, Ordering::Release);
        return current_optimal;
    }
}

/// Container to hold and dispatch jobs across a varying amount of threads. 
/// Can optionally be created in an uninitialized state, which can be initialized later with a specific number of threads.
/// Cannot be used in the uninitialized state. The thread count can be changed at runtime.
/// All operations on the job system are thread safe.
pub struct JobSystem {
    inner: UnsafeCell<MaybeUninit<Inner>>,
    is_initialized: bool
}

unsafe impl Send for JobSystem {}
unsafe impl Sync for JobSystem {}

impl JobSystem {
    /// Creates an uninitialized, thread safe JobSystem object. It does no allocation until `init()` is called.
    /// 
    /// # Panics
    /// 
    /// Use of the job system object AFTER `new_uninit()` without calling `init()` first will panic.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::job_system::system::JobSystem;
    /// let job_system = JobSystem::new_uninit();
    /// ```
    /// Is not initialized, so using it is invalid and will cause a panic.
    /// ``` should_panic
    /// # use gk_types_rs::job_system::system::JobSystem;
    /// let job_system = JobSystem::new_uninit();
    /// // Will panic cause not initialized. Must call job_system.init()
    /// job_system.run_job(|| 1);
    /// ```
    pub const fn new_uninit() -> JobSystem {
        return JobSystem { 
            inner: UnsafeCell::new(MaybeUninit::uninit()),
            is_initialized: false
        }
    }

    /// Create a new job system object given a specific number of threads.
    /// Ideally, the number of threads will be total system threads - 1. JobSystem is valid and usable in this state,
    /// so `init()` does not need to be called. `max_available_job_thread()` is a sensible default thread count.
    /// 
    /// # Panics
    /// 
    /// Panics if `thread_count` is 0.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::job_system::system::{JobSystem, max_available_job_threads};
    /// let job_system = JobSystem::new_init(max_available_job_threads());
    /// job_system.run_job(|| 1);
    /// ```
    pub fn new_init(thread_count: usize) -> JobSystem {
        debug_assert_ne!(thread_count, 0, "Cannot create a job system using 0 threads");
        let mut v: Vec<Box<JobThread>> = Vec::with_capacity(QUEUE_CAPACITY);
        for _ in 0..thread_count {
            v.push(JobThread::new());
        }
        return JobSystem { 
            inner: UnsafeCell::new(MaybeUninit::new(Inner {
                threads: v.into_boxed_slice(),
                thread_count,
                current_optimal_thread: AtomicUsize::new(0),
            })), 
            is_initialized: true
        }
    }

    /// Initializes an uninitalized JobSystem with a given thread count.
    /// Ideally, the number of threads will be total system threads - 1. `max_available_job_thread()` is a sensible default thread count.
    /// 
    /// # Panics
    /// 
    /// Cannot call `init()` if the JobSystem is already initialized. To modify thread counts,
    /// use `change_thread_count()`. Also panics if `thread_count` is 0.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::job_system::system::{JobSystem, max_available_job_threads};
    /// let mut job_system = JobSystem::new_uninit();
    /// job_system.init(max_available_job_threads());
    /// job_system.run_job(|| 1);
    /// ```
    /// Will panic if is already initialized.
    /// ``` should_panic
    /// # use gk_types_rs::job_system::system::{JobSystem, max_available_job_threads};
    /// let mut job_system = JobSystem::new_init(max_available_job_threads());
    /// // Will panic cause is already initialized. Similarly if `init()` gets called earlier.
    /// job_system.init(max_available_job_threads());
    /// ```
    pub fn init(&mut self, thread_count: usize) {
        debug_assert_ne!(thread_count, 0, "Cannot create a job system using 0 threads");
        assert!(!self.is_initialized, "JobSystem is already initialized");
        let mut v: Vec<Box<JobThread>> = Vec::with_capacity(QUEUE_CAPACITY);
        for _ in 0..thread_count {
            v.push(JobThread::new());
        }
        self.inner = UnsafeCell::new(MaybeUninit::new(Inner {
            threads: v.into_boxed_slice(),
            thread_count,
            current_optimal_thread: AtomicUsize::new(0),
        }));
        self.is_initialized = true;
        thread::yield_now();
    }

    /// Changes the thread count of an already initialized JobSystem with a given thread count.
    /// Ideally, the number of threads will be total system threads - 1. `max_available_job_thread()` is a sensible default thread count.
    /// 
    /// # Panics
    /// 
    /// Cannot call `change_thread_count()` if the JobSystem is not initialized. To initialize the JobSystem,
    /// use `JobSystem::new_init()` or `init()`. Also panics if `new_thread_count` is 0.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use gk_types_rs::job_system::system::JobSystem;
    /// let mut job_system = JobSystem::new_init(2);
    /// job_system.change_thread_count(4);
    /// assert_eq!(job_system.thread_count(), 4);
    /// ```
    /// As mentioned above, will panic if job system is not initialized.
    /// ``` should_panic
    /// # use gk_types_rs::job_system::system::JobSystem;
    /// let mut job_system = JobSystem::new_uninit();
    /// // Will panic because job system isn't initialized.
    /// job_system.change_thread_count(4);
    /// ```
    pub fn change_thread_count(&mut self, new_thread_count: usize) {
        debug_assert_ne!(new_thread_count, 0, "Cannot change JobSystem thread count using 0 threads");
        assert!(self.is_initialized, "JobSystem is not initialized");

        let inner = unsafe {self.inner.get_mut().assume_init_mut() };
        for job_thread in inner.threads.iter() {
            job_thread.wait();
        }

        let mut v: Vec<Box<JobThread>> = Vec::with_capacity(QUEUE_CAPACITY);
        for _ in 0..new_thread_count {
            v.push(JobThread::new());
        }
        inner.threads = v.into_boxed_slice();
        inner.thread_count = new_thread_count;
        inner.current_optimal_thread.store(0, Ordering::Release);
        thread::yield_now();
    }

    /// Queue and execute a job on one of the job threads. Automatic load balancing is done.
    /// ```
    /// # use gk_types_rs::job_system::{system::JobSystem, future::JobFuture};
    /// let job_system = JobSystem::new_init(2);
    /// let future1 = job_system.run_job(|| 123);
    /// let future2 = job_system.run_job(|| 456);
    /// assert_eq!(future1.wait(), 123);
    /// assert_eq!(future2.wait(), 456);
    /// ```
    pub fn run_job<T, F>(&self, func: F) -> JobFuture<T>
    where T: 'static, F: FnMut() -> T + 'static {
        debug_assert!(self.is_initialized, "JobSystem is not initialized. Please call init()");
        let job_thread = {
            let inner = unsafe { (&mut *self.inner.get()).assume_init_mut() };
            let optimal_thread_index = inner.get_optimal_thread_for_execution();
            &mut inner.threads[optimal_thread_index]
        };
        let future = (*job_thread).queue_job(func);
        (*job_thread).execute();
        return future;
    }

    /// Wait for all of the job threads to finish execution.
    /// After wait is called, it can be assumed that there are no active jobs running.
    /// 
    /// Note: It is technically possible for there to be jobs executing, 
    /// if the jobs created more jobs that happened to be on earlier threads.
    /// ```
    /// # use gk_types_rs::job_system::system::JobSystem;
    /// let job_system = JobSystem::new_init(2);
    /// job_system.run_job(|| std::thread::sleep(std::time::Duration::from_millis(10)));
    /// job_system.run_job(|| std::thread::sleep(std::time::Duration::from_millis(10)));
    /// job_system.wait();
    /// ```
    pub fn wait(&self) {
        debug_assert!(self.is_initialized, "JobSystem is not initialized. Please call init()");
        thread::yield_now();
        let inner = unsafe { (&mut *self.inner.get()).assume_init_mut() };
        for job_thread in inner.threads.iter() {
            job_thread.wait();
        }
    }

    /// Get the amount of threads this JobSystem has allocated. Is consistent until `change_thread_count()` is called.
    /// 
    /// # Example
    /// 
    /// ```
    /// # use gk_types_rs::job_system::system::JobSystem;
    /// let mut job_system = JobSystem::new_init(2);
    /// assert_eq!(job_system.thread_count(), 2);
    /// job_system.change_thread_count(4);
    /// assert_eq!(job_system.thread_count(), 4);
    /// ```
    pub fn thread_count(&self) -> usize {
        return unsafe {
            (&mut *self.inner.get()).assume_init_mut().thread_count
        }
    }
}

impl Drop for JobSystem {
    fn drop(&mut self) {
        if !self.is_initialized {
            return;
        }
        thread::yield_now();
        unsafe {(&mut *self.inner.get()).assume_init_drop()};
    }
}

pub fn max_available_job_threads() -> usize {
    return std::thread::available_parallelism().unwrap().get() - 1;
}