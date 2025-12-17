use anyhow::Result;
use blake3::Hasher;
use futures::{SinkExt, StreamExt};
use rayon::prelude::*;
use std::thread;
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot},
};
use tokio_util::codec::{Framed, LinesCodec};

pub const PREFIX_ZERO: &[u8] = &[0, 0, 0];

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("listen to: {}", addr);

    // create channel for tokio task <-> thread
    let (sender, mut receiver) = mpsc::unbounded_channel::<(String, oneshot::Sender<String>)>();

    // thread for cpu-bound task
    thread::spawn(move || {
        // read tokio task by blocking_recv, as now it's thread task
        while let Some((line, tx)) = receiver.blocking_recv() {
            // calc pow
            let result = match pow(&line) {
                Some((hash, nonce)) => format!("hash: {}, nonce: {}", hash, nonce),
                None => "Not found".to_string(),
            };
            // send result back
            if let Err(e) = tx.send(result) {
                println!("Failed to send: {}", e);
            }
        }
    });

    // use tokio task for IO-bound
    loop {
        let (stream, addr) = listener.accept().await?;
        println!("Accepted: {:?}", addr);
        let sender1 = sender.clone();
        tokio::spawn(async move {
            // LinesCodec to cut stream
            let framed = Framed::new(stream, LinesCodec::new());
            // split
            let (mut w, mut r) = framed.split();
            // for line in r.next().await {
            while let Some(Ok(line)) = r.next().await {
                // oneshot channel for each msg
                let (tx, rx) = oneshot::channel();
                sender1.send((line, tx))?;

                // recv result
                if let Ok(v) = rx.await {
                    w.send(format!("Pow calculated: {}", v)).await?;
                }
            }
            Ok::<_, anyhow::Error>(())
        });
    }
}

pub fn pow(s: &str) -> Option<(String, u32)> {
    let hasher = blake3_base_hash(s.as_bytes());
    // find nonce
    let nonce = (0..u32::MAX).into_par_iter().find_any(|n| {
        let hash = blake3_hash(hasher.clone(), n).as_bytes().to_vec();
        &hash[..PREFIX_ZERO.len()] == PREFIX_ZERO
    });
    // return
    nonce.map(|n| {
        let hash = blake3_hash(hasher, &n).to_hex().to_string();
        (hash, n)
    })
}

fn blake3_hash(mut hasher: blake3::Hasher, nonce: &u32) -> blake3::Hash {
    hasher.update(&nonce.to_be_bytes()[..]);
    hasher.finalize()
}

fn blake3_base_hash(data: &[u8]) -> Hasher {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher
}
