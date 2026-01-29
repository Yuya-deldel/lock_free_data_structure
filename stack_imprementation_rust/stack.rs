use std::ptr::null_mut;
use std::cell::UnsafeCell;
use std::arch::asm;

// AArch64 assembly 限定
// ABA 問題を回避するため、LL/SC assembly 命令によって実装する

#[repr(C)]          // assembly からアクセスするため、コンパイラによる定義順の入れ替えを阻止している
struct Node<T> {
    next: *mut Node<T>,
    data: T,
}

#[repr(C)]
pub struct StackHead<T> {
    head: *mut Node<T>,
}

impl<T> StackHead<T> {
    fn new() -> Self {
        StackHead { head: null_mut() }
    }

    pub fn push(&mut self, data: T) {
        let node = Box::new(Node {      // Node を heap 上に作成
            next: null_mut(),
            data: data,
        });
        // 生の pointer を usize として取り出す
        let ptr = Box::into_raw(node) as *mut u8 as usize; 
        let head = &mut self.head as *mut *mut Node<T> as *mut u8 as usize;

        unsafe {
            asm!("
                2:                                                      
                ldxr {next}, [{head}]           // load  next <- *head                       
                str {next}, [{ptr}]             // store *ptr <- next
                stlxr w10, {ptr}, [{head}]      // store *head <- ptr in Ordering::Release
                                                // store に成功したら w10 = 0, 失敗したら w10 = 1
                                                // head の'中身'が変わっていたら失敗する
                cbnz w10, 2b                    // if w10 != 0 then goto 2 
                                                // b は jump 先に対して後方に位置することを示す
            ", next = out(reg) _, ptr = in(reg) ptr, head = in(reg) head, out("w10") _,) 
            // 変数が入力なのか出力なのかを指定
        };
    }

    pub fn pop(&mut self) -> Option<T> {
        unsafe {
            let head = &mut self.head as *mut *mut Node<T> as *mut u8 as usize;
            let mut result: usize;      // pop した node への address

            asm!("
                2:
                ldaxr {result}, [{head}]        // load result <- *head in Ordering::Acquire
                cbnz {result}, 3f               // if result != NULL then goto 3
                                                // f は jump 先に対して前方に位置することを示す
                clrex                           // clear exclusive
                b 4f                            // jump 4

                3:                              // result != NULL case
                ldr {next}, [{result}]          // load next <- *result
                stxr w10, {next}, [{head}]      // store *head <- next; store に成功したら w10 = 0
                cbnz w10, 2b                    // if w10 != 0 then goto 2

                4:
            ", next = out(reg) _, result = out(reg) result, head = in(reg) head, out("w10") _);

            if result == 0 {
                return None;
            } else {
                let ptr = result as *mut u8 as *mut Node<T>;
                let head = Box::from_raw(ptr);
                return Some((*head).data);
            }
        }
    }
}

impl<T> Drop for StackHead<T> {
    fn drop(&mut self) {
        let mut node = self.head;
        while node != null_mut() {
            let box_ = unsafe {Box::from_raw(node)};
            node = box_.next;
        }
    }
}

pub struct Stack<T> {
    data: UnsafeCell<StackHead<T>>
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Stack { data: UnsafeCell::new(StackHead::new()) }
    }

    pub fn get_mut(&self) -> &mut StackHead<T> {
        unsafe {&mut *self.data.get()}
    }
}

unsafe impl<T> Sync for Stack<T> {}
unsafe impl<T> Send for Stack<T> {}