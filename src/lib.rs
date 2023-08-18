use std::{
    thread::JoinHandle,
    sync::{
        atomic::{
            AtomicBool,
            Ordering,
        },
        Mutex,
        Arc,
        Condvar,
    },
    thread,
    num::NonZeroUsize,
};

struct Signal {
    cond: Condvar,
    mutex: Mutex<()>,
}

impl Signal {
    fn new() -> Self {
        Signal {
            cond: Condvar::new(),
            mutex: Mutex::new(()),
        }
    }

    fn wait(&self) {
        let guard = self.mutex.lock().unwrap();
        let _unused_guard = self.cond.wait(guard).unwrap();
    }
}

pub struct CustomThreadPool {
    threads: Vec::<JoinHandle<()>>,
    tasks: Arc<Mutex<Vec::<Box<dyn FnOnce() + Send>>>>,
    shutdown: Arc<AtomicBool>,
    signal: Arc<Signal>,
}

impl CustomThreadPool {
    pub fn new(size: NonZeroUsize) -> Self {
        let mut thread_pool = CustomThreadPool {
            threads: Vec::new(),
            tasks: Arc::new(Mutex::new(Vec::new())),
            shutdown: Arc::new(AtomicBool::from(false)),
            signal: Arc::new(Signal::new()),
        };

        for i in 0..size.into() {
            let shutdown_clone = thread_pool.shutdown.clone();
            let tasks_clone = thread_pool.tasks.clone();
            let signal_clone = thread_pool.signal.clone();
            thread_pool.threads.push(
                thread::spawn(move || {
                    println!("Thread {i} started");

                    loop {
                        if shutdown_clone.load(Ordering::Acquire) {
                            println!("Thread {i} shutting down");
                            break;
                        }

                        let task =
                            {
                                //The lock will be held as long as the MutexGuard is alive. So I
                                // need to create a scope to make sure the lock is not held for the
                                // duration of the task being run.
                                let mut all_tasks = tasks_clone.lock().unwrap();

                                all_tasks.pop()
                            };

                        if let Some(t) = task {
                            println!("Thread {i} running task");
                            t();
                        } else {
                            println!("Thread {i} sleeping");
                            signal_clone.wait();
                        }
                    }
                })
            );
        }

        thread_pool
    }

    pub fn add_task<F>(&mut self, job: F) where F: FnOnce() + 'static + Send {
        {
            let mut unlocked_tasks = self.tasks.lock().unwrap();
            unlocked_tasks.push(Box::from(job));
        }

        self.signal.cond.notify_one();
    }

    pub fn shutdown(&mut self) {
        {
            let mut unlocked_tasks = self.tasks.lock().unwrap();
            unlocked_tasks.clear();
        }
        self.shutdown.store(true, Ordering::Release);
        self.signal.cond.notify_all();
    }
}

impl Drop for CustomThreadPool {
    fn drop(&mut self) {
        self.shutdown();
        while let Some(item) = self.threads.pop() {
            item.join().unwrap();
        }
        println!("ThreadPool Shut Down");
    }
}
