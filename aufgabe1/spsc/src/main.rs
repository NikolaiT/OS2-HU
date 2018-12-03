use std::thread;
//use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::usize;
use std::collections::VecDeque;

/*
	Ideas and code snippets taken from:

	https://stackoverflow.com/questions/47092072/one-mutable-borrow-and-multiple-immutable-borrows
	https://gist.github.com/LeoTindall/e6d40782b05dc8ac40faf3a0405debd3
	https://doc.rust-lang.org/std/sync/struct.Mutex.html
*/

#[derive(Debug)]
pub struct Error {
	message: String
}

#[derive(Debug)]
pub struct SendError<T>(pub T);

#[derive(Debug)]
pub struct RecvError {
	message: String
}

// All three of these types are wrapped around a generic type T.
// T is required to be Send (a marker trait automatically implemented when
// it is safe to do so) because it denotes types that are safe to move between
// threads, which is the whole point of the WorkQueue.
// For this implementation, T is required to be Copy as well, for simplicity.

/// A generic work queue for work elements which can be trivially copied.
/// Any producer of work can add elements and any worker can consume them.
/// WorkQueue derives Clone so that it can be distributed among threads.

#[derive(Clone)]
pub struct Producer<T: Send + Copy> {
	queue: Arc<Mutex<VecDeque<T>>>,
}

#[derive(Clone)]
pub struct Consumer<T: Send + Copy> {
	queue: Arc<Mutex<VecDeque<T>>>,
}

impl<T: Send + Copy> Producer<T> {

	pub fn new(capacity: usize) -> Self {
		Self { queue: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))) }
	}

	pub fn send(&self, value: T) -> Result<(), SendError<T>> {
		// try to get a lock to the mutex...
		if let Ok(mut queue) = self.queue.lock() {
			queue.push_back(value);
			Ok(())
		} else {
			panic!("Producer::send() could not lock mutex.");
		}
	}

	pub fn capacity(&self) -> Result<usize, Error> {
		if let Ok(queue) = self.queue.lock() {
			let capacity = queue.capacity();
			Ok(capacity)
		} else {
			panic!("Producer::send() could not lock mutex.");
		}
	}

	pub fn size(&self) -> Result<usize, Error> {
		if let Ok(queue) = self.queue.lock() {
			let len = queue.len();
			Ok(len)
		} else {
			panic!("Producer::send() could not lock mutex.");
		}
	}
}

impl<T: Send + Copy> Consumer<T> {

	pub fn new(capacity: usize) -> Self {
		Self { queue: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))) }
	}

	pub fn recv(&self) -> Result<T, RecvError> {
		// A lot is going on here. self.queue is an Arc of Mutex. Arc can deref
		// into its internal type, so we can call the methods of that inner
		// type (Mutex) without dereferencing, so this is like
		//      *(self.inner).lock()
		// but doesn't look awful. Mutex::lock() returns a
		// Result<MutexGuard<VecDeque<T>>>.
		let maybe_queue = self.queue.lock();

		if let Ok(mut queue) = maybe_queue {

			let mut result;

			// loop until pop_front() actually returns a value
			loop {
				// unpack the option and return a Result
				// in case of an error from pop_front(), return
				// a descriptive error message.
				result = queue.pop_front();
				match result {
					None => {}
					Some(_res) => {
						break;
					}
				}
			}

			match result {
				None => Err(RecvError{ message: "Consumer::recv() pop_front() returned None.".to_string() }),
				Some(result) => Ok(result)
			}


		} else {
			Err(RecvError{ message: "Consumer::recv() could not lock mutex.".to_string() })
		}
	}

	pub fn capacity(&self) -> Result<usize, Error> {
		if let Ok(queue) = self.queue.lock() {
			let capacity = queue.capacity();
			Ok(capacity)
		} else {
			panic!("Producer::send() could not lock mutex.");
		}
	}

	pub fn size(&self) -> Result<usize, Error> {
		if let Ok(queue) = self.queue.lock() {
			let len = queue.len();
			Ok(len)
		} else {
			panic!("Producer::send() could not lock mutex.");
		}
	}
}

pub fn channel<T: Send + Copy>(capacity: usize) -> (Producer<T>, Consumer<T>) {

	let queue = Arc::new(Mutex::new(VecDeque::with_capacity(capacity)));

	(
		Producer {
			queue: queue.clone(),
		},
		Consumer {
			queue: queue.clone(),
		}
	)
}

fn main() {
	// start a producer thread that sends the values 1..count
	// and start a consumer thread that consumes
	let (px, cx) = channel(64);
	let count = 30;
	
	let producer_thread = thread::spawn(move || {
		for i in 1..count {
			px.send(i).unwrap();
		}
	});

	let consumer_thread = thread::spawn(move || {
		let mut sum = 0;
		for _i in 1..count {
			match cx.recv() {
				Ok(val)  => {
					println!("Receiving from producer: {:?}", val);
					sum += val;
				},
				Err(e) => println!("Error: {:?}", e),
			};
		}

		sum
	});

	producer_thread.join().unwrap();

	let sum = consumer_thread.join().unwrap();
	println!("Summing over {} values yields the sum {}", count, sum);
}


/*
 * Tests.
 */

#[cfg(test)]
mod tests {

	use super::*;
	use std::thread;

	#[test]
	fn test_consumer_pop() {
		let capacity: usize = 100;
		let (px, cx) = channel(capacity);

		for i in 0..9 {
			px.send(i);
			assert_eq!(px.capacity().unwrap(), capacity.next_power_of_two()-1);
			assert_eq!(px.size().unwrap(), i+1);
		}

		for i in 0..9 {
			assert_eq!(cx.size().unwrap(), 9-i);
			let t = cx.recv().unwrap();
			assert_eq!(cx.capacity().unwrap(), capacity.next_power_of_two()-1);
			assert_eq!(cx.size().unwrap(), 9-i-1);
			assert_eq!(t, i);
		}
	}

	#[test]
	fn queue_length_is_accurate() {
		let (px, cx) = channel(100);
		assert_eq!(0, cx.size().unwrap());
		for i in 0..11 {
			px.send(i);
			assert_eq!(i+1, cx.size().unwrap());
		}
	}

	#[test]
	fn threaded_queue_multiple_producer_single_consumer() {
		let (px1, cx) = channel(100);
		let px2 = px1.clone();

		thread::spawn(move || {
			px1.send(1).unwrap();
		});

		thread::spawn(move|| {
			px2.send(1).unwrap();
		});

		for _ in 0 .. 1 {
			assert_eq!(1, cx.recv().unwrap());
		}
	}

	#[test]
	fn test_threaded() {
		let capacity: usize = 64;
		let (px, cx) = channel(capacity);

		let producer_thread = thread::spawn(move || {
			for i in 0..1000 {
				px.send(i);
			}
		});

		let consumer_thread = thread::spawn(move || {
			for i in 0..1000 {
				match cx.recv() {
					Ok(val)  => {
						assert_eq!(val, i);
					},
					Err(e) => {
						println!("Error: {:?}", e)
					},
				};
			}
		});
	}

	extern crate time;
	use self::time::PreciseTime;


	#[test]
	#[ignore]
	fn bench_spsc_throughput() {
		let iterations: i64 = 2i64.pow(20);

		let (px, cx) = channel(512);

		let start = PreciseTime::now();
		for i in 0..iterations as usize {
			px.send(i);
		}
		let t = cx.recv().unwrap();
		assert_eq!(t, 0);
		let end = PreciseTime::now();
		let throughput =
			(iterations as f64 / (start.to(end)).num_nanoseconds().unwrap() as f64) * 1000000000f64;
		println!(
			"Spsc Throughput: {:.2}/s -- (iterations: {} in {} ns)",
			throughput,
			iterations,
			(start.to(end)).num_nanoseconds().unwrap()
		);
	}

	// we can either take the normal streaming channel mpsc::channel
	// or the mpsc::sync_channel
	// see here: https://doc.rust-lang.org/std/sync/mpsc/
	use std::sync::mpsc::sync_channel;
	use std::sync::mpsc::channel as mpsc_channel;

	#[test]
	#[ignore]
	fn bench_mpsc_stdlib_throughput() {
		let iterations: i64 = 2i64.pow(20);

		let (tx, rx) = mpsc_channel();

		let start = PreciseTime::now();
		for i in 0..iterations as usize {
			tx.send(i).unwrap();
		}
		let t = rx.recv().unwrap();
		assert_eq!(t, 0);
		let end = PreciseTime::now();
		let throughput =
			(iterations as f64 / (start.to(end)).num_nanoseconds().unwrap() as f64) * 1000000000f64;
		println!(
			"MPSC Stdlib Throughput: {:.2}/s -- (iterations: {} in {} ns)",
			throughput,
			iterations,
			(start.to(end)).num_nanoseconds().unwrap()
		);
	}

}