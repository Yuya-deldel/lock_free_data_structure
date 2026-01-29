use std::ptr::null_mut;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering::{Relaxed, Release, Acquire};

// lock をせずにデータを授受できるデータ構造
// pointer を atomic に変更することで実現している
// この実装では ABA 問題が発生する

struct Node<T> {
    next: AtomicPtr<Node<T>>,
    data: T,
}

pub struct StackBad<T> {
    head: AtomicPtr<Node<T>>,
}

impl<T> StackBad<T> {
    pub fn new() -> Self {
        StackBad { head: AtomicPtr::new(null_mut()) }
    }

    pub fn push(&self, data: T) {
        let node = Box::new(Node {      // Node を heap 上に作成
            next: AtomicPtr::new(null_mut()),
            data: data,
        });
        let ptr = Box::into_raw(node);      // 生の pointer を取り出す

        unsafe {
            loop {
                let head = self.head.load(Relaxed);
                (*ptr).next.store(head, Relaxed);       // 追加する node の next が head を指すようにする
                // head の値が変わっていなければ ptr に更新
                if let Ok(_) = self.head.compare_exchange_weak(head, ptr, Release, Relaxed) {
                    break;
                }
            }
        } 
    }

    pub fn pop(&self) -> Option<T> {
        unsafe {
            loop {
                let head = self.head.load(Relaxed);
                if head == null_mut() {
                    return None;
                }

                let next = (*head).next.load(Relaxed);

                // ここで他のスレッドが pop pop push したときに ABA 問題が発生する
                
                if let Ok(_) = self.head.compare_exchange_weak(head, next, Acquire, Relaxed) {
                    let box_ = Box::from_raw(head);
                    return Some((*box_).data);
                }
            }
        }
    }
}

impl<T> Drop for StackBad<T> {
    fn drop(&mut self) {
        let mut node = self.head.load(Relaxed);
        while node != null_mut() {
            let box_ = unsafe { Box::from_raw(node) };      // pointer を Box に戻す -> Box の drop trait により drop
            node = box_.next.load(Relaxed);
        }
    }
}