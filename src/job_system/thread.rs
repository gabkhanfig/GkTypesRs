#[allow(invalid_reference_casting)]

use std::{sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Condvar, Mutex}, thread};

use super::{job_container::JobContainer, future::{JobFuture, WithinJobFuture}, ring_queue::JobRingQueue, active_jobs::ActiveJobs};

pub struct JobThread {
    is_executing: AtomicBool,
    is_pending_kill: AtomicBool,
    should_execute: AtomicBool,

    pub queued_job_count: AtomicUsize,

    thread: Option<thread::JoinHandle<()>>,
    cond_var: (Mutex<bool>, Condvar),

    queue: Mutex<JobRingQueue>,
    active_work: Mutex<ActiveJobs>
}

struct JobThreadHandle(*mut JobThread);

unsafe impl Send for JobThreadHandle {}
unsafe impl Sync for JobThreadHandle {}

impl JobThread {
    /// Makes a new JobThread object. It is set to a valid state, 
    /// where the job thread is sleeping until it has work queued and execute() is called.
    /// ```
    /// # use gk_types_rs::job_system::thread::JobThread;
    /// let job_thread = JobThread::new();
    /// ```
    pub fn new() -> Box<JobThread> {
        let mut job_thread = Box::new(JobThread { 
            is_executing: AtomicBool::new(false), 
            is_pending_kill: AtomicBool::new(false), 
            should_execute: AtomicBool::new(false),
            queued_job_count: AtomicUsize::new(0), 
            cond_var: (Mutex::new(true), Condvar::new()), 
            queue: Mutex::new(JobRingQueue::new()), 
            active_work: Mutex::new(ActiveJobs::new()),
            thread: Option::None, 
        });

        let thread_ptr: JobThreadHandle = JobThreadHandle(&mut *job_thread as *mut JobThread);
        
        job_thread.thread = Option::Some(
            thread::spawn(move || {
                let _ = &thread_ptr; // Will allow the pointer shenanigans
                unsafe {
                    while (*thread_ptr.0).is_pending_kill.load(Ordering::Acquire) == false {
                        let (lock, cvar) = &mut (*thread_ptr.0).cond_var;

                        let count = {
                            // scoped to release lock
                            (*thread_ptr.0).queue.lock().unwrap().length                      
                        };
                        if count > 0 {
                            (*thread_ptr.0).execute_queued_jobs();
                            continue;
                        }

                        (*thread_ptr.0).is_executing.store(false, Ordering::Release);
                        {
                            let _result = cvar.wait_while(
                                lock.lock().unwrap(),
                                |_| (*thread_ptr.0).should_execute.load(Ordering::Relaxed) == false
                            ).unwrap();
                        }
                        (*thread_ptr.0).execute_queued_jobs();
                    }
                }
            })
        ); 

        return job_thread;
    }

    /// Adds a job to this job thread's queue, returning a future for completion.
    /// Will not execute the queue until JobThread::execute() is called.
    /// ```
    /// # use gk_types_rs::job_system::thread::JobThread;
    /// # use gk_types_rs::job_system::future::JobFuture;
    /// let mut job_thread = JobThread::new();
    /// // Will not execute until JobThread::execute() is called
    /// let future = job_thread.queue_job(|| 10);
    /// ```
    pub fn queue_job<T, F>(&mut self, mut func: F) -> JobFuture<T>
    where T: 'static, F: FnMut() -> T + 'static {
        let (wait_future, in_job_future) = WithinJobFuture::<T>::new();
        let job = JobContainer::new(move ||
            in_job_future.set(func())
        );

        {
            let mut queue_lock = self.queue.lock().unwrap();
            (*queue_lock).push(job);
            self.queued_job_count.fetch_add(1, Ordering::Release);
        }

        return wait_future;
    }

    /// Executes the jobs that are queued.
    /// ```
    /// # use gk_types_rs::job_system::thread::JobThread;
    /// # use gk_types_rs::job_system::future::JobFuture;
    /// let mut job_thread = JobThread::new();
    /// let future1 = job_thread.queue_job(|| 10);
    /// let future2 = job_thread.queue_job(|| 20);
    /// // Jobs will execute here
    /// job_thread.execute();
    /// let num1 = future1.wait();
    /// let num2 = future2.wait();
    /// assert_eq!(num1, 10);
    /// assert_eq!(num2, 20);
    /// ```
    pub fn execute(&mut self) {
        if self.is_executing.load(Ordering::Acquire) == true { 
            // should already be looping the execution, in which if it has any queued jobs, it will execute them.
            return;
        }
        self.should_execute.store(true, Ordering::Release);
        self.cond_var.1.notify_one();
        self.is_executing.store(true, Ordering::Release);
    }

    /// Waits until execution of the entire queue is completed. 
    /// If more jobs are queued while it's executing, wait will continue waiting.
    /// ```
    /// # use gk_types_rs::job_system::thread::JobThread;
    /// # use std::{thread, time::Duration};
    /// let mut job_thread = JobThread::new();
    /// job_thread.queue_job(|| thread::sleep(Duration::from_millis(5)));
    /// job_thread.queue_job(|| thread::sleep(Duration::from_millis(5)));
    /// job_thread.execute();
    /// job_thread.wait();
    /// ```
    pub fn wait(&self) {
        while self.is_executing.load(Ordering::Acquire) == true {
            thread::yield_now();
        }
    }

    /// Atomically get the number of jobs queued. Does not get the queue mutex.
    /// ```
    /// # use gk_types_rs::job_system::thread::JobThread;
    /// # use std::{thread, time::Duration};
    /// let mut job_thread = JobThread::new();
    /// for i in 0..10 {
    ///     job_thread.queue_job(move || i);
    /// }
    /// assert_eq!(job_thread.queued_count(), 10);
    /// ```
    pub fn queued_count(&self) -> usize {
        return self.queued_job_count.load(Ordering::Acquire);
    }

    /// Atomically check if the job thread is executing. Useful for optimal scheduling.
    /// ```
    /// # use gk_types_rs::job_system::thread::JobThread;
    /// # use std::{thread, time::Duration};
    /// let mut job_thread = JobThread::new();
    /// job_thread.queue_job(|| thread::sleep(Duration::from_millis(5)));
    /// job_thread.execute();
    /// assert!(job_thread.is_executing());
    /// ```
    pub fn is_executing(&self) -> bool {
        return self.is_executing.load(Ordering::Acquire);
    }

    fn execute_queued_jobs(&mut self) {
        let mut active_lock = self.active_work.lock().unwrap();
        {
            let mut queue_lock = self.queue.lock().unwrap();
            self.queued_job_count.store(0, Ordering::Release);
            (*active_lock).collect_jobs(&mut *queue_lock);
            // queue lock is unlocked here.
        }
        (*active_lock).invoke_all_jobs();
    }

}

impl Drop for JobThread {
    fn drop(&mut self) {
        self.wait();
        self.is_pending_kill.store(true, Ordering::SeqCst);
        self.queued_job_count.store(isize::MAX as usize, Ordering::Release); // some insanely huge value that couldn't happen naturally. Not usize::MAX to not cause issues with incrementing
        self.should_execute.store(true, Ordering::Release);
        self.cond_var.1.notify_one();
        let thread = std::mem::take(&mut self.thread).unwrap();
        thread.join().expect("failed to join job thread");

        // self.wait();
        // self.is_pending_kill.store(true, Ordering::Release);

        // let job_count = self.queued_job_count.swap(isize::MAX as usize, Ordering::SeqCst);
        // if job_count > 0 {
        //     self.execute();
        // }

        // self.cond_var.1.notify_one();

        
        // //self.queued_job_count.store(isize::MAX as usize, Ordering::Release); // some insanely huge value that couldn't happen naturally. Not usize::MAX to not cause issues with incrementing
        
        // let thread = std::mem::take(&mut self.thread).unwrap();
        // thread.join().expect("failed to join job thread");
    }
}