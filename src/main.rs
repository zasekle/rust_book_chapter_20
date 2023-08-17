use std::{
    fs,
    io::{
        prelude::*,
        BufReader,
    },
    net::{
        TcpListener,
        TcpStream,
    },
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
};

struct Signal {
    cond: Condvar,
    mutex: Mutex<()>,
}

impl Signal {
    fn new() -> Self {
        Signal {
            cond : Condvar::new(),
            mutex: Mutex::new(()),
        }
    }

    fn wait(&mut self) {
        let mut guard = self.mutex.lock().unwrap();
        self.cond.wait(guard).unwrap();
    }
}

//TODO: move to a library
struct ThreadPool {
    threads: Vec::<JoinHandle<()>>,
    tasks: Arc<Mutex<Vec::<Box<dyn FnOnce() + Send>>>>,
    shutdown: Arc<AtomicBool>,
    signal: Arc<Signal>,
}

impl ThreadPool {
    fn run_thread(i: i32) {
        println!("running run_thread() {i}");
    }

    pub fn new(size: isize) -> Self {
        let mut thread_pool = ThreadPool {
            threads: Vec::new(),
            tasks: Arc::new(Mutex::new(Vec::new())),
            shutdown: Arc::new(AtomicBool::from(false)),
            signal: Arc::new(Signal::new()),
        };

        for i in 0..size {
            let shutdown_clone = thread_pool.shutdown.clone();
            let tasks_clone = thread_pool.tasks.clone();
            let signal_clone = thread_pool.signal.clone();
            thread_pool.threads.push(
                thread::spawn(move || {
                    println!("Thread started\n");

                    loop {
                        if shutdown_clone.load(Ordering::Acquire) {
                            break;
                        }
                        // signal_clone.wait();
                        //TODO: need to wait at the condition variable

                        let task =
                            {
                                //The lock will be held as long as the MutexGuard is alive. So I
                                // need to create a scope to make sure the lock is not held for the
                                // duration of the task being run.
                                let mut all_tasks = tasks_clone.lock().unwrap();

                                all_tasks.pop()
                            };

                        if let Some(t) = task {
                            t();
                        }
                    }
                })
            );
        }

        thread_pool
    }

    pub fn add_task<F>(&mut self, job: F) where F: FnOnce() + 'static {
        let mut unlocked_tasks = self.tasks.lock().unwrap();
        unlocked_tasks.push(Box::from(job));
        //TODO: notify a thread
    }

    pub fn shutdown(&mut self) {
        //TODO: Probably want to call this on deconstruction too.
        self.shutdown.store(true, Ordering::Release);
        //TODO: Probably need to notify all the condition variables, then wait (join) here or in the deconstructor
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let thread_pool = ThreadPool::new(2);

    // let mut thread_pool= Vec::new();

    // thread_pool.push(|info| println!("hello: {}", info));

    // let thread = thread::spawn(|| println!("hello"));

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    let (status_line, file_name) =
        if request_line == "GET / HTTP/1.1" {
            ("HTTP/1.1 200 OK", "hello.html")
        } else {
            ("HTTP/1.1 404 NOT FOUND", "404.html")
        };

    let contents = fs::read_to_string(format!("./src/{file_name}")).unwrap();
    let length = contents.len();

    let response = format!(
        "{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}"
    );

    stream.write_all(response.as_bytes()).unwrap();
}

//TODO: need a pool of threads to call from, maybe a vector of threads
//TODO: probably just want a queue of tasks and the thread pool pops from them as it needs right?