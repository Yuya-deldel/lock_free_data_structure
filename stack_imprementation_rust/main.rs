#![feature(asm)]

// AArch64 Only

mod stack_with_aba_problem;
mod stack;

use std::sync::Arc;

const NUM_LOOP: usize = 1000000;
const NUM_THREADS: usize = 4;

fn main() {
    let stack = Arc::new(stack::Stack::<usize>::new());
    let mut to_be_joined = Vec::new();

    for i in 0..NUM_THREADS {
        let stack = stack.clone();
        let t = std::thread::spawn(move || {
            if i & 1 == 0 {     // 偶数スレッドは push
                for j in 0..NUM_LOOP {
                    let k = i * NUM_LOOP + j;
                    stack.get_mut().push(k);
                    println!("push: {}", k);
                }
                println!("finished push: #{}", i);
            } else {
                for _ in 0..NUM_LOOP {
                    loop {
                        if let Some(k) = stack.get_mut().pop() {
                            println!("pop: {}", k);
                            break;
                        }
                    }
                }
                println!("finished pop: #{}", i);
            }
        });
        to_be_joined.push(t);
    }

    for t in to_be_joined {
        t.join().unwrap();
    }

    assert!(stack.get_mut().pop() == None);
}
