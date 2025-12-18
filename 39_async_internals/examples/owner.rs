use std::pin::Pin;

fn main() {
    let s = "hello".to_string();
    println!("addr of s(stack): {:p}", &s);
    let s = Pin::new(s);
    println!("addr of pinned s: {:p}", &s);
}
