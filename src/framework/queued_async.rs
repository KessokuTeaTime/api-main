use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    hash::Hash,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicU8, Ordering},
    },
};

use parking_lot::Mutex;
use tracing::{error, info, warn};

use crate::framework::{
    state::{State, retry_if_possible},
    transaction::Transaction,
};

#[derive(Debug, Default)]
struct BusinessHolder {
    lock: tokio::sync::Mutex<()>,
    latest_payload_index: AtomicU8,
}

#[derive(Debug)]
pub struct QueuedAsyncFrameworkContext {
    pub payload_display: String,
    pub index: u8,
    pub holder: Arc<BusinessHolder>,
}

impl QueuedAsyncFrameworkContext {
    pub fn should_exit(&self) -> bool {
        let latest_payload_index = &self.holder.latest_payload_index.load(Ordering::SeqCst);
        let result = self.index < latest_payload_index - 1;
        if result {
            warn!(
                "current payload index ({}) is falling behind the latest one ({latest_payload_index}), exiting deployment with {}!",
                &self.index, &self.payload_display
            );
        }
        result
    }
}

pub struct QueuedAsyncFramework<'a, ID, V>
where
    ID: Eq + Hash,
    V: Clone + Send,
{
    businesses: LazyLock<Mutex<HashMap<ID, Arc<BusinessHolder>>>>,
    transaction_builder:
        Box<dyn for<'r> Fn(&'r QueuedAsyncFrameworkContext) -> Transaction<'r, V, State<()>> + 'a>,
}

impl<ID, V> Debug for QueuedAsyncFramework<'_, ID, V>
where
    ID: Eq + Hash + Debug,
    V: Clone + Send + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueuedAsyncFramework")
            .field("businesses", &self.businesses)
            .field("transaction_builder", &"TransactionBuilder")
            .finish()
    }
}

impl<'a, ID, V> QueuedAsyncFramework<'a, ID, V>
where
    ID: Eq + Hash,
    V: Clone + Send,
{
    pub fn new<B>(builder: B) -> Self
    where
        B: for<'r> Fn(&'r QueuedAsyncFrameworkContext) -> Transaction<'r, V, State<()>> + 'a,
    {
        QueuedAsyncFramework {
            businesses: LazyLock::new(|| Mutex::new(HashMap::new())),
            transaction_builder: Box::new(builder),
        }
    }
}

impl<ID, V> QueuedAsyncFramework<'_, ID, V>
where
    ID: Eq + Hash,
    V: Clone + Send,
{
    pub async fn run(&self, id: ID, payload: V)
    where
        V: Display,
    {
        let holder = self.businesses.lock().entry(id).or_default().clone();
        let index = holder.latest_payload_index.fetch_add(1, Ordering::SeqCst);
        let context: QueuedAsyncFrameworkContext = QueuedAsyncFrameworkContext {
            payload_display: format!("{}", &payload),
            index,
            holder: holder.clone(),
        };

        info!("starting transaction loop with payload {}…", &payload);
        let mut retry: u8 = 0;
        let _guard = holder.lock.lock().await;

        loop {
            if context.should_exit() {
                break;
            }

            let transaction = (self.transaction_builder)(&context);
            match transaction.run(payload.clone()).await {
                State::Success(_) => {
                    holder
                        .latest_payload_index
                        .store(u8::default(), Ordering::SeqCst);
                    info!("transaction succeed with payload {}!", &payload)
                }
                State::Retry => match retry_if_possible(&mut retry) {
                    Ok(_) => continue,
                    Err(_) => break,
                },
                State::Stop => error!("transaction failure with payload {}!", &payload),
            }
        }
    }
}
