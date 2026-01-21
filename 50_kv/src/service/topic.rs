use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use dashmap::DashMap;
use dashmap::DashSet;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use crate::CommandResponse;
use crate::KvError;
use crate::Value;

const BROADCAST_CAPACITY: usize = 128;

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

fn get_next_subscription_id() -> u32 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

pub trait Topic: Send + Sync + 'static {
    /// sub topic
    fn subscribe(self, name: String) -> mpsc::Receiver<Arc<CommandResponse>>;
    /// unsub topic
    fn unsubscribe(self, name: String, id: u32) -> Result<u32, KvError>;
    /// publish
    fn publish(self, name: String, value: Arc<CommandResponse>);
}

#[derive(Default)]
pub struct Broadcaster {
    /// all topics
    topics: DashMap<String, DashSet<u32>>,
    /// id - Sender
    subscriptions: DashMap<u32, mpsc::Sender<Arc<CommandResponse>>>,
}

//
impl Topic for Arc<Broadcaster> {
    #[instrument(name = "topic_subscribe", skip_all)]
    fn subscribe(self, name: String) -> mpsc::Receiver<Arc<CommandResponse>> {
        let id = {
            let entry = self.topics.entry(name).or_default();
            let id = get_next_subscription_id();
            entry.value().insert(id);
            id
        };

        // new channel
        let (tx, rx) = mpsc::channel(BROADCAST_CAPACITY);

        let v: Value = (id as i64).into();

        // send
        let tx1 = tx.clone();
        tokio::spawn(async move {
            if let Err(e) = tx1.send(Arc::new(v.into())).await {
                // TODO: 这个很小概率 ，但目前我们没有售后
                warn!("Failed to send subscription id {}, Error: {:?}", id, e);
            }
        });

        self.subscriptions.insert(id, tx);
        debug!("Subscription {} is added", id);

        rx
    }

    #[instrument(name = "topic_unsubscribe", skip_all)]
    fn unsubscribe(self, name: String, id: u32) -> Result<u32, KvError> {
        match self.remove_subscription(name, id) {
            Some(id) => Ok(id),
            None => Err(KvError::NotFound(format!("subscription {}", id))),
        }
    }

    #[instrument(name = "topic_publish", skip_all)]
    fn publish(self, name: String, value: Arc<CommandResponse>) {
        tokio::spawn(async move {
            let mut ids = vec![];
            match self.topics.get(&name) {
                Some(topic) => {
                    // 复制整个topic下所有的 subscription_id
                    // 这里每个id是u32, 如果1个topic有10k的订阅，复制成本是40k堆内存
                    let subscription = topic.value().clone();

                    // 尽快释放锁
                    drop(topic);

                    for id in subscription.into_iter() {
                        if let Some(tx) = self.subscriptions.get(&id) {
                            if let Err(e) = tx.send(value.clone()).await {
                                warn!("Publish to {} failed, error: {:?}", id, e);
                                ids.push(id);
                            }
                        }
                    }
                }
                None => {}
            }
            for id in ids {
                self.remove_subscription(name.clone(), id);
            }
        });
    }
}

impl Broadcaster {
    pub fn remove_subscription(&self, name: String, id: u32) -> Option<u32> {
        if let Some(v) = self.topics.get_mut(&name) {
            // if topics contain, remove it
            v.remove(&id);

            if v.is_empty() {
                info!("Topic {:?} is deleted", &name);
                drop(v);
                self.topics.remove(&name);
            }
        }

        debug!("Subscription {} is removed", id);

        // remove from table
        self.subscriptions.remove(&id).map(|(id, _)| id)
    }
}

//

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use crate::assert_res_ok;

    use super::*;

    #[tokio::test]
    async fn pub_sub_should_work() {
        let b = Arc::new(Broadcaster::default());
        let lobby = "lobby".to_string();

        // subscribe
        let mut stream1 = b.clone().subscribe(lobby.clone());
        let mut stream2 = b.clone().subscribe(lobby.clone());

        // publish
        let v: Value = "hello".into();
        b.clone().publish(lobby.clone(), Arc::new(v.clone().into()));

        // subscribers 应该能收到 publish 的数据
        let id1: i64 = stream1.recv().await.unwrap().as_ref().try_into().unwrap();
        let id2: i64 = stream2.recv().await.unwrap().as_ref().try_into().unwrap();

        assert!(id1 != id2);

        let res1 = stream1.recv().await.unwrap();
        let res2 = stream2.recv().await.unwrap();

        assert_eq!(res1, res2);
        assert_res_ok(&res1, &[v.clone()], &[]);

        // 如果 subscriber 取消订阅，则收不到新数据
        let res = b.clone().unsubscribe(lobby.clone(), id1 as _).unwrap();
        assert_eq!(res, id1 as _);

        // publish
        let v: Value = "world".into();
        b.clone().publish(lobby.clone(), Arc::new(v.clone().into()));

        assert!(stream1.recv().await.is_none());
        let res2 = stream2.recv().await.unwrap();
        assert_res_ok(&res2, &[v.clone()], &[]);
    }
}
