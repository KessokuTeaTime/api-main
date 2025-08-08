//! A framework that loops transactions until the max retry times is reached, or a stop signal is received, or a value is returned.

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

/// Unwraps a [`State`] to control the loop of a [`QueuedAsyncFramework`]. This macro accepts a [`State`] value, returns the current scope if the value is either [`State::Retry`] or [`State::Stop`], and exposes the data if the value is [`State::Success`].
///
/// # Examples
///
/// ```rust
/// let value = unwrap!(State::Success(42));
/// assert!(value == 42);
///
/// fn scope() -> State<()> {
///     // This line returns the function with a `State::<()>::Stop` immediately
///     let value: i32 = unwrap!(State::Stop);
///
///     // This line will never be executed
///     State::Success(())
/// }
///
/// assert!(scope() == State::Stop);
/// ```
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

/// A framework that loops transactions until the max retry times is reached, or a stop signal is received, or a value is returned.
///
/// This framework ensures that the latest business is always executed. The ongoing business should check itself constantly in case a newer business arrives. This is achieved through an index that grows with collapsing businesses, and the [`QueuedAsyncFrameworkContext::check`] function along with the [`unwrap`] macro.
///
/// See: [`example`], [`unwrap`]
#[derive(Debug, Default)]
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
    /// Creates a [`QueuedAsyncFramework`].
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
    /// Runs transactions asynchronously with a payload. The `id` is used to distinguish between businesses.
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

#[tokio::test]
async fn example() {
    // Defines a framework
    // This leverages `LazyLock` to generate a static value
    static FRAMEWORK: LazyLock<QueuedAsyncFramework<i32>> =
        LazyLock::new(QueuedAsyncFramework::new);

    // Runs the transaction inside the framework
    FRAMEWORK
        .run(42, String::from("payload"), |cx| {
            // Pinboxes the transaction and clone the context
            Box::pin(transaction(cx.clone()))
        })
        .await;

    async fn transaction(cx: QueuedAsyncFrameworkContext<String>) -> State<()> {
        // Checks if a newer business exist, and stops executing if so
        unwrap!(cx.check());

        // Any logic returning a `State` can be unwrapped...
        let greeting = unwrap!(greet().await);
        // ...while `State::Retry` and `State::Stop` can control the loop directly
        assert!(greeting == "42!");

        // To exit successfully...
        State::Success(())
    }

    async fn greet() -> State<String> {
        State::Success(String::from("42!"))
    }
}
