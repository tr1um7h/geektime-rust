use crate::*;

impl CommandService for Hget {
    fn execute(self, store: &impl Storage) -> CommandResponse {
        match store.get(&self.table, &self.key) {
            Ok(Some(v)) => v.into(),
            Ok(None) => KvError::NotFound(self.table, self.key).into(),
            Err(e) => e.into(),
        }
    }
}

impl CommandService for Hgetall {
    fn execute(self, store: &impl Storage) -> CommandResponse {
        match store.get_all(&self.table) {
            Ok(v) => v.into(),
            Err(e) => e.into(),
        }
    }
}

impl CommandService for Hset {
    fn execute(self, store: &impl Storage) -> CommandResponse {
        match self.pair {
            Some(v) => match store.set(&self.table, v.key, v.value.unwrap_or_default()) {
                Ok(Some(v)) => v.into(),
                Ok(None) => Value::default().into(),
                Err(e) => e.into(),
            },
            None => KvError::InvalidCommand(format!("{:?}", self)).into(),
        }
    }
}

impl CommandService for Hmget {
    fn execute(self, store: &impl Storage) -> CommandResponse {
        let mut ret = Vec::<Value>::new();
        for k in self.keys {
            match store.get(&self.table, &k) {
                Ok(Some(v)) => ret.push(v.into()),
                Ok(None) => ret.push(Value::default().into()),
                Err(e) => return e.into(),
            }
        }
        println!("ret: {:?}", ret);
        ret.into()
    }
}

impl CommandService for Hmset {
    fn execute(self, store: &impl Storage) -> CommandResponse {
        let mut ret = Vec::<Kvpair>::new();
        for v in self.pairs {
            println!("v: {:?}", v);
            match store.set(&self.table, v.key.clone(), v.value.unwrap_or_default()) {
                Ok(Some(o)) => ret.push(Kvpair::new(v.key, o)),
                Ok(None) => ret.push(Kvpair::new(v.key, Value::default().into())),
                Err(e) => return e.into(),
            }
        }
        println!("ret: {:?}", ret);
        ret.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hset_should_work() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hset("t1", "hello", "world".into());
        let res = dispatch(cmd.clone(), &store);
        assert_res_ok(res, &[Value::default()], &[]);

        let res = dispatch(cmd, &store);
        assert_res_ok(res, &["world".into()], &[]);
    }

    #[test]
    fn hget_should_work() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hset("score", "u1", 10.into());
        dispatch(cmd, &store);
        let cmd = CommandRequest::new_hget("score", "u1");
        let res = dispatch(cmd, &store);
        assert_res_ok(res, &[10.into()], &[]);
    }

    #[test]
    fn hget_with_non_exist_key_should_return_404() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hget("score", "u1");
        let res = dispatch(cmd, &store);
        assert_res_error(res, 404, "Not found");
    }

    #[test]
    fn hgetall_should_work() {
        let store = MemTable::new();
        let cmds = vec![
            CommandRequest::new_hset("score", "u1", 10.into()),
            CommandRequest::new_hset("score", "u2", 8.into()),
            CommandRequest::new_hset("score", "u3", 11.into()),
            CommandRequest::new_hset("score", "u1", 6.into()),
        ];
        for cmd in cmds {
            dispatch(cmd, &store);
        }

        let cmd = CommandRequest::new_hgetall("score");
        let res = dispatch(cmd, &store);
        let pairs = &[
            Kvpair::new("u1", 6.into()),
            Kvpair::new("u2", 8.into()),
            Kvpair::new("u3", 11.into()),
        ];
        assert_res_ok(res, &[], pairs);
    }

    #[test]
    fn hmget_should_work() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hset("score", "u1", 10.into());
        let _ = dispatch(cmd.clone(), &store);
        let cmd = CommandRequest::new_hset("score", "u2", 20.into());
        let _ = dispatch(cmd.clone(), &store);
        let cmd = CommandRequest::new_hget("score", "u1");
        let res = dispatch(cmd, &store);
        assert_res_ok(res, &[10.into()], &[]);
        let cmd = CommandRequest::new_hget("score", "u2");
        let res = dispatch(cmd, &store);
        assert_res_ok(res, &[20.into()], &[]);

        let cmd = CommandRequest::new_hmget("score", vec!["u1".into(), "u2".into()]);
        let res = dispatch(cmd, &store);
        assert_res_ok(res, &[10.into(), 20.into()], &[]);
    }

    #[test]
    fn hmset_should_work() {
        let store = MemTable::new();
        let cmd_hmset = CommandRequest::new_hmset(
            "score",
            vec![Kvpair::new("u1", 10.into()), Kvpair::new("u2", 20.into())],
        );
        let res = dispatch(cmd_hmset.clone(), &store);
        assert_res_ok(
            res,
            &[],
            &[
                Kvpair::new("u1", Value::default()),
                Kvpair::new("u2", Value::default()),
            ],
        );

        let cmd = CommandRequest::new_hget("score", "u1");
        let res = dispatch(cmd.clone(), &store);
        assert_res_ok(res, &[10.into()], &[]);
        let cmd = CommandRequest::new_hget("score", "u2");
        let res = dispatch(cmd.clone(), &store);
        assert_res_ok(res, &[20.into()], &[]);

        let res = dispatch(cmd_hmset, &store);
        assert_res_ok(
            res,
            &[],
            &[Kvpair::new("u1", 10.into()), Kvpair::new("u2", 20.into())],
        );
    }

    // TODO:
    // 1. HMGET、HMSET、HDEL、HMDEL、HEXIST、HMEXIST
    // 2. get_iter
    // 3. service notif
    // 4. persist
    // 5. return error ASAP
    // 6. add rocksdb
}
