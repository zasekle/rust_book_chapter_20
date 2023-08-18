use std::{
    thread::JoinHandle,
    sync::{
        atomic::{
            AtomicBool,
            Ordering,
        },
        Mutex,
        Arc,
        mpsc::Sender,
        mpsc,
    },
    thread,
    num::NonZeroUsize,
};

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct CustomThreadPool {
    threads: Vec::<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
    sender: Sender<Job>,
}

impl CustomThreadPool {
    pub fn new(size: NonZeroUsize) -> Self {
        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut thread_pool = CustomThreadPool {
            threads: Vec::new(),
            shutdown: Arc::new(AtomicBool::from(false)),
            sender,
        };

        for i in 0..size.into() {
            let shutdown_clone = Arc::clone(&thread_pool.shutdown);
            let receiver_clone = Arc::clone(&receiver);

            thread_pool.threads.push(
                thread::spawn(move || {
                    println!("Thread {i} started");
                    loop {
                        let job = receiver_clone.lock().unwrap().recv().unwrap();

                        if shutdown_clone.load(Ordering::Acquire) {
                            break
                        }

                        println!("Thread {i} running");
                        job();
                    }
                })
            );
        }

        thread_pool
    }

    pub fn add_task<F>(&mut self, task: F) where F: FnOnce() + 'static + Send {
        let task_ptr = Box::new(task);
        let _unused_lock = self.sender.send(task_ptr);
    }

    pub fn shutdown(&mut self) {
        self.shutdown.store(true, Ordering::Release);

        for _ in 0..self.threads.len() {
            let shutdown_job = Box::new(|| {});
            let _unused_lock = self.sender.send(shutdown_job);
        }
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
