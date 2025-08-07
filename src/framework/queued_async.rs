use super::{State, retry_if_possible};

use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    hash::Hash,
    pin::Pin,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicU8, Ordering},
    },
};

use tracing::{error, info, warn};

#[macro_export]
macro_rules! unwrap {
    ($expr:expr) => {
        match $expr {
            $crate::framework::State::Success(v) => v,
            $crate::framework::State::Retry => return $crate::framework::State::<()>::Retry,
            $crate::framework::State::Stop => return $crate::framework::State::<()>::Stop,
        }
    };
}

pub use unwrap;

#[derive(Debug, Default)]
struct BusinessHolder {
    lock: tokio::sync::Mutex<()>,
    latest_payload_index: AtomicU8,
}

#[derive(Debug, Clone)]
pub struct QueuedAsyncFrameworkContext<V>
where
    V: Display,
{
    pub index: u8,
    pub payload: V,
    holder: Arc<BusinessHolder>,
}

impl<V> QueuedAsyncFrameworkContext<V>
where
    V: Display,
{
    pub fn check(&self) -> State<()> {
        let latest_payload_index = &self.holder.latest_payload_index.load(Ordering::SeqCst);
        if self.index < latest_payload_index - 1 {
            warn!(
                "current payload index ({}) is falling behind the latest one ({latest_payload_index}), exiting deployment with {}!",
                &self.index, &self.payload
            );
            State::Success(())
        } else {
            State::Stop
        }
    }
}

#[derive(Debug)]
pub struct QueuedAsyncFramework<ID>
where
    ID: Eq + Hash,
{
    businesses: LazyLock<parking_lot::Mutex<HashMap<ID, Arc<BusinessHolder>>>>,
}

impl<ID> QueuedAsyncFramework<ID>
where
    ID: Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            businesses: LazyLock::new(|| parking_lot::Mutex::new(HashMap::new())),
        }
    }
}

impl<ID> QueuedAsyncFramework<ID>
where
    ID: Eq + Hash,
{
    pub async fn run<V, F>(&self, id: ID, payload: V, f: F)
    where
        V: Clone + Display + Send,
        F: Fn(QueuedAsyncFrameworkContext<V>) -> Pin<Box<dyn Future<Output = State<()>> + Send>>
            + Send
            + Sync,
    {
        let holder = self.businesses.lock().entry(id).or_default().clone();
        let index = holder.latest_payload_index.fetch_add(1, Ordering::SeqCst);
        let context: QueuedAsyncFrameworkContext<V> = QueuedAsyncFrameworkContext {
            index,
            payload: payload.clone(),
            holder: holder.clone(),
        };

        info!("starting transaction with payload {payload}…",);
        let mut retry: u8 = 0;
        let _guard = holder.lock.lock().await;

        loop {
            match context.check() {
                State::Retry => continue,
                State::Stop => break,
                _ => {}
            }

            match f(context.clone()).await {
                State::Success(_) => {
                    holder
                        .latest_payload_index
                        .store(u8::default(), Ordering::SeqCst);
                    info!("transaction succeed with payload {payload}!",);
                }
                State::Retry => match retry_if_possible(&mut retry) {
                    Ok(_) => continue,
                    Err(_) => break,
                },
                State::Stop => error!("transaction failure with payload {payload}!",),
            }
        }
    }
}
