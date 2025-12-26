mod error;
mod network;
mod pb;
mod service;
mod storage;

pub use error::KvError;
pub use network::*;
pub use pb::abi::*;
pub use service::*;
pub use storage::*;

// TODO:
// run `cargo run --example gen_cert` if cert is out-dated
// * 现在我们的系统对 Pub/Sub 已经有比较完整的支持，但你有没有注意到，有一个潜在的内存泄漏的 bug。如果客户端 A subscribe 了 Topic M，但客户端意外终止，且随后也没有任何人往 Topic M publish 消息。这样，A 的 subscription 就一直放在表中。你能做一个 GC 来处理这种情况么？
// * Redis 还支持 PSUBSCRIBE，也就是说除了可以 subscribe “chat” 这样固定的 topic，还可以是 “chat.*”，一并订阅所有 “chat”、“chat.rust”、“chat.elixir” 。想想看，如果要支持 PSUBSCRIBE，你该怎么设计 Broadcaster 里的两张表？
