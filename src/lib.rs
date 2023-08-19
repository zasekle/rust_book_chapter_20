use std::{
    thread::JoinHandle,
    sync::{
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
    threads: Vec<JoinHandle<()>>,
    sender: Option<Sender<Job>>,
}

impl CustomThreadPool {
    pub fn new(size: NonZeroUsize) -> Self {
        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut thread_pool = CustomThreadPool {
            threads: Vec::new(),
            sender: Some(sender),
        };

        for i in 0..size.into() {
            let receiver_clone = Arc::clone(&receiver);

            thread_pool.threads.push(
                thread::spawn(move || loop {
                    println!("Thread {i} waiting for job.");
                    let job_result = receiver_clone.lock().unwrap().recv();

                    match job_result {
                        Ok(job) => {
                            println!("Thread {i} running.");
                            job();
                        }
                        Err(_) => {
                            println!("Thread {i} shutting down.");
                            break;
                        }
                    }
                })
            );
        }

        thread_pool
    }

    pub fn add_task<F>(&mut self, task: F) where F: FnOnce() + 'static + Send {
        let task_ptr = Box::new(task);
        if let Some(sender) = &self.sender {
            sender.send(task_ptr).unwrap();
        }
    }

    pub fn shutdown(&mut self) {
        drop(self.sender.take());
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
