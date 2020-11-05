pub mod logger {
    use slog::{Drain, Never, SendSyncRefUnwindSafeDrain};

    pub fn drain() -> impl 'static + SendSyncRefUnwindSafeDrain<Err = Never, Ok = ()> {
        let decorator = slog_term::TermDecorator::new().stderr().build();
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        drain
    }
}
