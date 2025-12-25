use crate::{
    command_request::RequestData, CommandRequest, CommandResponse, KvError, MemTable, Storage,
};
use std::sync::Arc;
use tracing::debug;

mod command_service;

/// 对 Command 的处理的抽象
pub trait CommandService {
    /// 处理 Command，返回 Response
    fn execute(self, store: &impl Storage) -> CommandResponse;
}

// TODO: 1. exit in advance 2. support rocksdb

/// 事件通知（不可变事件）
pub trait Notify<Arg, T, E> {
    fn notify(&self, arg: &Arg) -> Result<T, E>;
}

/// 事件通知（可变事件）
pub trait NotifyMut<Arg, T, E> {
    fn notify(&self, arg: &mut Arg) -> Result<T, E>;
}

impl<Arg> Notify<Arg, (), KvError> for Vec<fn(&Arg) -> Result<(), KvError>> {
    #[inline]
    fn notify(&self, arg: &Arg) -> Result<(), KvError> {
        for f in self {
            match f(arg) {
                Ok(_) => {}
                Err(e) => return Err(KvError::Internal(e.to_string())),
            }
        }
        Ok(())
    }
}

impl<Arg> NotifyMut<Arg, (), KvError> for Vec<fn(&mut Arg) -> Result<(), KvError>> {
    #[inline]
    fn notify(&self, arg: &mut Arg) -> Result<(), KvError> {
        for f in self {
            match f(arg) {
                Ok(_) => {}
                Err(e) => return Err(KvError::Internal(e.to_string())),
            }
        }
        Ok(())
    }
}

/// Service 数据结构
pub struct Service<Store = MemTable> {
    inner: Arc<ServiceInner<Store>>,
}

impl<Store> Clone for Service<Store> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Service 内部数据结构
pub struct ServiceInner<Store> {
    store: Store,
    on_received: Vec<fn(&CommandRequest) -> Result<(), KvError>>,
    on_executed: Vec<fn(&CommandResponse) -> Result<(), KvError>>,
    on_before_send: Vec<fn(&mut CommandResponse) -> Result<(), KvError>>,
    on_after_send: Vec<fn() -> Result<(), KvError>>,
}

impl<Store: Storage> ServiceInner<Store> {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            on_received: Vec::new(),
            on_executed: Vec::new(),
            on_before_send: Vec::new(),
            on_after_send: Vec::new(),
        }
    }

    pub fn fn_received(mut self, f: fn(&CommandRequest) -> Result<(), KvError>) -> Self {
        self.on_received.push(f);
        self
    }

    pub fn fn_executed(mut self, f: fn(&CommandResponse) -> Result<(), KvError>) -> Self {
        self.on_executed.push(f);
        self
    }

    pub fn fn_before_send(mut self, f: fn(&mut CommandResponse) -> Result<(), KvError>) -> Self {
        self.on_before_send.push(f);
        self
    }

    pub fn fn_after_send(mut self, f: fn() -> Result<(), KvError>) -> Self {
        self.on_after_send.push(f);
        self
    }
}

impl<Store: Storage> From<ServiceInner<Store>> for Service<Store> {
    fn from(inner: ServiceInner<Store>) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

impl<Store: Storage> Service<Store> {
    pub fn execute(&self, cmd: CommandRequest) -> Result<CommandResponse, KvError> {
        debug!("Got request: {:?}", cmd);
        self.inner.on_received.notify(&cmd)?;
        let mut res = dispatch(cmd, &self.inner.store);
        debug!("Executed response: {:?}", res);
        self.inner.on_executed.notify(&res)?;
        self.inner.on_before_send.notify(&mut res)?;
        if !self.inner.on_before_send.is_empty() {
            debug!("Modified response: {:?}", res);
        }

        Ok(res)
    }
}

// 从 Request 中得到 Response，目前处理 HGET/HGETALL/HSET
pub fn dispatch(cmd: CommandRequest, store: &impl Storage) -> CommandResponse {
    match cmd.request_data {
        Some(RequestData::Hget(param)) => param.execute(store),
        Some(RequestData::Hgetall(param)) => param.execute(store),
        Some(RequestData::Hmget(param)) => param.execute(store),
        Some(RequestData::Hset(param)) => param.execute(store),
        Some(RequestData::Hmset(param)) => param.execute(store),
        Some(RequestData::Hdel(param)) => param.execute(store),
        Some(RequestData::Hmdel(param)) => param.execute(store),
        Some(RequestData::Hexist(param)) => param.execute(store),
        Some(RequestData::Hmexist(param)) => param.execute(store),
        None => KvError::InvalidCommand("Request has no data".into()).into(),
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use http::StatusCode;
    use tracing::info;

    use super::*;
    use crate::{MemTable, Value};

    #[test]
    fn service_should_works() {
        // 我们需要一个 service 结构至少包含 Storage
        let service: Service = ServiceInner::new(MemTable::default()).into();

        // service 可以运行在多线程环境下，它的 clone 应该是轻量级的
        let cloned = service.clone();

        // 创建一个线程，在 table t1 中写入 k1, v1
        let handle = thread::spawn(move || {
            let res = cloned.execute(CommandRequest::new_hset("t1", "k1", "v1".into()));
            assert_res_ok(res.unwrap(), &[Value::default()], &[]);
        });
        handle.join().unwrap();

        // 在当前线程下读取 table t1 的 k1，应该返回 v1
        let res = service.execute(CommandRequest::new_hget("t1", "k1"));
        assert_res_ok(res.unwrap(), &["v1".into()], &[]);
    }

    #[test]
    fn event_registration_should_work() {
        fn b(cmd: &CommandRequest) -> Result<(), KvError> {
            info!("Got {:?}", cmd);
            Ok(())
        }
        fn c(res: &CommandResponse) -> Result<(), KvError> {
            info!("{:?}", res);
            Ok(())
        }
        fn d(res: &mut CommandResponse) -> Result<(), KvError> {
            res.status = StatusCode::CREATED.as_u16() as _;
            Ok(())
        }
        fn e() -> Result<(), KvError> {
            info!("Data is sent");
            Ok(())
        }

        let service: Service = ServiceInner::new(MemTable::default())
            // .fn_received(|_: &CommandRequest| Err(KvError::Internal("error".to_string())))
            .fn_received(b)
            .fn_executed(c)
            .fn_before_send(d)
            .fn_after_send(e)
            .into();

        let res = service.execute(CommandRequest::new_hset("t1", "k1", "v1".into()));
        // assert!(res.is_err());
        let res = res.unwrap();
        assert_eq!(res.status, StatusCode::CREATED.as_u16() as _);
        assert_eq!(res.message, "");
        assert_eq!(res.values, vec![Value::default()]);
    }
}

#[cfg(test)]
use crate::{Kvpair, Value};

// 测试成功返回的结果
#[cfg(test)]
pub fn assert_res_ok(mut res: CommandResponse, values: &[Value], pairs: &[Kvpair]) {
    res.pairs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(res.status, 200);
    assert_eq!(res.message, "");
    assert_eq!(res.values, values);
    assert_eq!(res.pairs, pairs);
}

// 测试失败返回的结果
#[cfg(test)]
pub fn assert_res_error(res: CommandResponse, code: u32, msg: &str) {
    assert_eq!(res.status, code);
    assert!(res.message.contains(msg));
    assert_eq!(res.values, &[]);
    assert_eq!(res.pairs, &[]);
}
