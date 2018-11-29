use std::thread;
use std::sync::mpsc::channel;

fn shared() {
    // Create a shared channel that can be sent along from many threads
    // where tx is the sending half (tx for transmission), and rx is the receiving
    // half (rx for receiving).
    let (tx, rx) = channel();
    for i in 0..10 {
        let tx = tx.clone();
        thread::spawn(move || {
            tx.send(i).unwrap();
        });
    }

    for _ in 0..10 {
        let j = rx.recv().unwrap();
        println!("j is {}", j);
        assert!(0 <= j && j < 10);
    }
}

fn simple() {
    // Create a simple streaming channel
    let (tx, rx) = channel();
    thread::spawn(move || {
        tx.send(10).unwrap();
    });
    assert_eq!(rx.recv().unwrap(), 10);
}

fn main() {
    shared();
}
