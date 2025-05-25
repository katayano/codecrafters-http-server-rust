//! This programs is created by following the Rust book
//! https://doc.rust-jp.rs/book-ja/ch20-02-multithreaded.html
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

/// Trait for calling a closure while moving closure from Box<T>
/// https://doc.rust-jp.rs/book-ja/ch20-02-multithreaded.html
trait FnBox {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<F>) {
        // Call the function inside the box
        (*self)();
    }
}

type Job = Box<dyn FnBox + Send + 'static>;

impl ThreadPool {
    /// Initialize the thread pool with the given size
    /// size is the number of threads in the pool
    /// # Panics
    /// Panics if size is 0
    pub fn new(size: usize) -> Self {
        assert!(size > 0, "Thread pool size must be greater than 0");

        let (sender, receiver) = mpsc::channel();

        // Create a reveiver that is shared among all workers
        // Wrap the receiver in an Arc and Mutex to share it among threads
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        // Execute the given function in a thread
        let job = Box::new(f);
        // Send the job to the workers
        self.sender.send(job).unwrap();
    }
}

/// Struct that represents a worker thread
/// Each worker will receive jobs from the thread pool
struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let thread = thread::spawn(move || {
            loop {
                // Lock the receiver to get a job
                let job = receiver.lock().unwrap().recv().unwrap();
                println!("Worker {} got a job; executing.", id);
                // Execute the job
                job.call_box();
            }
        });

        Worker { id, thread }
    }
}
