//! The single-poll executor a native test runs a handler on.

/// Runs a handler to completion on this thread and answers what it returned.
///
/// Every hook and every registered handler is `async`, because in a session a
/// host verb really does suspend the guest until the engine answers. A test
/// needs some way to run one. This is the whole of the async support this
/// module offers, and the reason a mod's test suite pulls in no runtime.
///
/// It is also how an author's own hook is driven. An author hook is a plain
/// `async fn` that the generated dispatch table reaches by
/// [`HookId`](crate::HookId), so a test calls the function and runs it here
/// rather than firing an edge at a dispatch table.
///
/// ```
/// use ironlark::server::prelude::*;
/// use ironlark::testing::{FakePlayer, block_on, context};
///
/// // Nothing declared is in play, so this handler needs no declaration.
/// async fn greeting(_ctx: Context, player: Player) -> String {
///     format!("welcome, {player}")
/// }
///
/// let player = FakePlayer::new(SessionId::new(4)).into_player();
/// let said = block_on(greeting(context(Tick::new(1), Tick::new(1)), player));
///
/// assert_eq!(said, "welcome, player#4");
/// ```
///
/// # A single poll, not a runtime
///
/// It polls once, with a waker that does nothing, and takes the answer. There
/// is no loop and nothing to wake it, so a future that answers `Pending` panics
/// with "a native test future suspended; only a session can resume it".
///
/// That is sound here and only here. On the build machine every host verb
/// answers from a local buffer without yielding, so a handler built out of this
/// crate's verbs always completes on the first poll. A handler that awaits
/// something else — a queue, a timer, a task another thread must finish — is
/// what trips the panic, and the panic is right, because a test that appeared
/// to pass would have proved nothing about the handler.
///
/// The same handler in a session genuinely suspends and is resumed by the
/// engine. Completing on the first poll is a property of the double, never a
/// property of the mod.
pub fn block_on<F: core::future::Future>(fut: F) -> F::Output {
    use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    fn no_op(_: *const ()) {}
    fn clone(p: *const ()) -> RawWaker {
        RawWaker::new(p, &VTABLE)
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, no_op, no_op, no_op);
    let waker = unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) };
    let mut cx = Context::from_waker(&waker);
    let mut fut = core::pin::pin!(fut);
    match fut.as_mut().poll(&mut cx) {
        Poll::Ready(out) => out,
        Poll::Pending => panic!("a native test future suspended; only a session can resume it"),
    }
}
