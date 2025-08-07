use std::fmt::Debug;

use crate::framework::state::State;

pub struct Transaction<'a, V, R> {
    function: Box<dyn FnOnce(V) -> R + 'a>,
}

impl<V, R> Debug for Transaction<'_, V, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("function", &"TransactionFunction")
            .finish()
    }
}

impl<'a, V, R> Transaction<'a, V, R> {
    pub fn create<F>(op: F) -> Self
    where
        F: FnOnce(V) -> R + 'a,
    {
        Transaction {
            function: Box::new(op),
        }
    }

    pub fn run(self, payload: V) -> R {
        (self.function)(payload)
    }

    pub fn next<F, N>(self, op: F) -> Transaction<'a, V, N>
    where
        F: FnOnce(R) -> N + 'a,
        V: 'a,
        R: 'a,
    {
        Transaction {
            function: Box::new(|v| op((self.function)(v))),
        }
    }
}

impl<'a, V, R> Transaction<'a, V, State<R>> {
    pub fn map_next<F, N>(self, op: F) -> Transaction<'a, V, State<N>>
    where
        F: FnOnce(R) -> State<N> + 'a,
        V: 'a,
        R: 'a,
    {
        Transaction {
            function: Box::new(|v| match (self.function)(v) {
                State::Success(r) => op(r),
                State::Retry => State::Retry,
                State::Stop => State::Stop,
            }),
        }
    }
}
