use crate::framework::state::State;

use std::{fmt::Debug, pin::Pin};

pub mod global;

#[macro_export]
macro_rules! transaction {
    (|$($name:ident: $type: ty),+| -> $output:ty; $body:expr) => {
        move |$($name: $type,)+| -> std::pin::Pin<Box<dyn Future<Output=$output> + Send>> {
            Box::pin(async move {
                $body
            })
        }
    };
    (|$($name:ident: $type: ty),+| -> $output:ty; $($($var:expr),+ =>)? await $func:path) => {
        transaction!(|$($name: $type),+| -> $output; $func($($($var,)+)? $($name),+).await)
    }
}

pub use transaction;

pub trait TransactionFunction<'a, V, R> = AsyncFnOnce<(V,), Output = R, CallOnceFuture = Pin<Box<dyn Future<Output = R> + Send + 'a>>>
    + Send;

pub struct Transaction<'a, V, R>
where
    V: Send,
{
    pub(crate) function: Box<dyn TransactionFunction<'a, V, R> + 'a>,
}

impl<V, R> Debug for Transaction<'_, V, R>
where
    V: Send,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("function", &"TransactionFunction")
            .finish()
    }
}
impl<'a, V, R> Transaction<'a, V, R>
where
    V: Send,
{
    pub fn create<F, In>(op: F) -> Self
    where
        F: TransactionFunction<'a, In, R> + 'a,
        V: Into<In> + 'a,
    {
        Transaction {
            function: Box::new(transaction! {
                |v: V| -> R;
                {
                    op(v.into()).await
                }
            }),
        }
    }
}

impl<'a, V, R> Transaction<'a, V, R>
where
    V: Send,
{
    pub async fn run(self, payload: V) -> R {
        (self.function).async_call_once((payload,)).await
    }

    pub fn next<F, In, Out>(self, op: F) -> Transaction<'a, V, Out>
    where
        F: TransactionFunction<'a, In, Out> + 'a,
        V: 'a,
        R: Into<In> + 'a,
    {
        Transaction {
            function: Box::new(transaction! {
                |v: V| -> Out;
                {
                    let r = (self.function).async_call_once((v,)).await;
                    op(r.into()).await
                }
            }),
        }
    }
}

impl<'a, V, R> Transaction<'a, V, R>
where
    V: Send + 'a,
    R: 'a,
{
    pub fn next_become<N>(self, value: N) -> Transaction<'a, V, N>
    where
        N: Send + 'a,
    {
        Transaction {
            function: Box::new(transaction! {
                |v: V| -> N;
                {
                    drop((self.function).async_call_once((v,)).await);
                    value
                }
            }),
        }
    }
}

impl<'a, V, R> Transaction<'a, V, State<R>>
where
    V: Send + 'a,
    R: Send + 'a,
{
    pub fn map_next<F, In, Out>(self, op: F) -> Transaction<'a, V, State<Out>>
    where
        F: TransactionFunction<'a, In, State<Out>> + 'a,
        R: Into<In>,
    {
        Transaction {
            function: Box::new(transaction! {
                |v: V| -> State<Out>;
                {
                    let r = (self.function).async_call_once((v,)).await;
                    match r {
                        State::Success(r) => op(r.into()).await,
                        State::Retry => State::Retry,
                        State::Stop => State::Stop,
                    }
                }
            }),
        }
    }
}
impl<'a, V, R> Transaction<'a, V, State<R>>
where
    V: Send + 'a,
    R: 'a,
{
    pub fn map_next_become<N>(self, value: N) -> Transaction<'a, V, State<N>>
    where
        N: Send + 'a,
    {
        Transaction {
            function: Box::new(transaction! {
                |v: V| -> State<N>;
                {
                    let r = (self.function).async_call_once((v,)).await;
                    match r {
                        State::Success(_) => State::Success(value),
                        State::Retry => State::Retry,
                        State::Stop => State::Stop,
                    }
                }
            }),
        }
    }
}

impl<'a, V, R> Transaction<'a, V, R>
where
    V: Send + 'a,
    R: Send + 'a,
{
    pub fn and_then<N>(self, transaction: Transaction<'a, R, N>) -> Transaction<'a, V, N>
    where
        N: 'a,
    {
        self.next(transaction.function)
    }
}
