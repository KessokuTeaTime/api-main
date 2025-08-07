use std::{fmt::Debug, pin::Pin};

use crate::framework::state::State;

#[macro_export]
macro_rules! transaction {
    (|$($name:ident: $type: ty),+| -> $output:ty {$body:expr}) => {
        move |$($name: $type,)+| -> std::pin::Pin<Box<dyn Future<Output=$output> + Send>> {
            Box::pin(async move {
                $body
            })
        }
    };
}

pub struct Transaction<'a, V, R>
where
    V: Send,
{
    function: Box<
        dyn AsyncFnOnce<
                (V,),
                Output = R,
                CallOnceFuture = Pin<Box<dyn Future<Output = R> + Send + 'a>>,
            > + Send
            + 'a,
    >,
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
    pub fn create<F>(op: F) -> Self
    where
        F: AsyncFnOnce<
                (V,),
                Output = R,
                CallOnceFuture = Pin<Box<dyn Future<Output = R> + Send + 'a>>,
            > + Send
            + 'a,
    {
        Transaction {
            function: Box::new(op),
        }
    }

    pub async fn run(self, payload: V) -> R {
        (self.function).async_call_once((payload,)).await
    }

    pub fn next<F, N>(self, op: F) -> Transaction<'a, V, N>
    where
        F: AsyncFnOnce<
                (R,),
                Output = N,
                CallOnceFuture = Pin<Box<dyn Future<Output = N> + Send + 'a>>,
            > + Send
            + 'a,
        V: 'a,
        R: 'a,
    {
        Transaction {
            function: Box::new(transaction!(|v: V| -> N {
                {
                    let r = (self.function).async_call_once((v,)).await;
                    op(r).await
                }
            })),
        }
    }
}

impl<'a, V, R> Transaction<'a, V, State<R>>
where
    V: Send,
    R: Send,
{
    pub fn map_next<F, N>(self, op: F) -> Transaction<'a, V, State<N>>
    where
        F: AsyncFnOnce<
                (R,),
                Output = State<N>,
                CallOnceFuture = Pin<Box<dyn Future<Output = State<N>> + Send + 'a>>,
            > + Send
            + 'a,
        V: 'a,
        R: 'a,
    {
        Transaction {
            function: Box::new(transaction!(|v: V| -> State<N> {
                {
                    let r = (self.function).async_call_once((v,)).await;
                    match r {
                        State::Success(r) => op(r).await,
                        State::Retry => State::Retry,
                        State::Stop => State::Stop,
                    }
                }
            })),
        }
    }
}
