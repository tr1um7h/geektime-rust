// cargo +nightly run --example generator

// #![feature(generators, generator_trait)]
#![feature(coroutines)]
#![feature(coroutine_trait)]
#![feature(stmt_expr_attributes)]

// use std::ops::{Generator, GeneratorState};
use core::ops::{Coroutine, CoroutineState};
use std::pin::Pin;

fn main() {
    let mut generator = #[coroutine]
    || {
        yield 1;
        return "foo";
    };

    match Pin::new(&mut generator).resume(()) {
        CoroutineState::Yielded(v) => {
            eprintln!("got {}", v);
        }
        _ => panic!("unexpected return from resume"),
    }
    match Pin::new(&mut generator).resume(()) {
        CoroutineState::Complete(v) => {
            eprintln!("got {}", v);
        }
        _ => panic!("unexpected return from resume"),
    }
}
