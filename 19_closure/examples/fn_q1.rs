use std::{collections::HashMap, mem::size_of_val};
fn main() {
    let name = String::from("Tyr");
    let vec = vec!["Rust", "Elixir", "Javascript"];
    let v = &vec[..];
    let data = (1, 2, 3, 4);
    let c = move || {
        println!("data: {:?}", data);
        println!("v: {:?}, name: {:?}", v, name.clone());
    };
    println!("{}", size_of_val(&c));
    c();

    // 请问在这里，还能访问 name 么？为什么？
    // println!("{}", name);
}
