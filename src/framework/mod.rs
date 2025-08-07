pub mod queued_async;
pub mod state;
pub mod transaction;

pub trait FrameworkContext {
    fn payload_display(&self) -> &str;
}
