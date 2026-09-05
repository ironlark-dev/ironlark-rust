//! The addressed act's declaration, and the one awaited verb that reads it.

use crate::context::Context;
use crate::error::{Error, Refusal, Result};
use crate::ids::RequestId;
use crate::protocol::{decode, encode, resolved};
use crate::server::player::Player;
use core::future::Future;
use prost::Message;

/// A declared request: the client half asks, the server half answers, and the
/// answer may be no.
///
/// [`protocol!`](macro@crate::protocol) writes this impl on the request's own
/// payload type, from an `rpc` inside a `service` block in the mod's
/// `protocol.proto`. The service's name says which half answers, and
/// `service Client` is refused where it is written: that direction is not
/// served, and the schema is where an author is told so.
///
/// One name, two types. The type carrying this impl is what the caller hands
/// over, and [`Response`](Self::Response) is what comes back. The response may
/// be any message the schema defines, a type that declares a signal of its own
/// included — reusing one rather than minting a near-copy is the intended
/// thing, and the `Response` suffix is a guideline for when a fresh type is
/// right.
///
/// It is the one awaited act in the SDK, and the one place a
/// [`Result`](crate::Result) is an ANSWER somebody waited for rather than a
/// report. [`client::request`](crate::client::request) is the asking end and
/// [`respond`](Self::respond) — a method on this type — the answering one. That
/// handler returns `Result<Response, Refusal>`, and both arms ride the same
/// wire back, so a [`Refusal`](crate::Refusal) reaches the caller as an
/// [`Error`](crate::Error) of kind [`Refused`](crate::ErrorKind::Refused)
/// carrying the handler's own words.
///
/// A request is offered to the mod that declared it and to no other, so two
/// authors may both ship an `open` without either answering for the other.
///
/// ```
/// use ironlark::protocol::RequestSpec;
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // The hidden line is `ironlark::protocol!("../protocol.proto")` over the
/// // door schema:
/// //
/// //     service Server { rpc Open(OpenRequest) returns (Latch); }
/// //     message OpenRequest { uint32 force = 1; }
/// //
/// // Out comes `protocol::OpenRequest` carrying the impl below. `Latch` is
/// // the answer here and declares a signal of its own in the same schema.
/// assert_eq!(<protocol::OpenRequest as RequestSpec>::NAME, "open");
///
/// // The response type is the schema's too, readable as an associated type.
/// fn answers_a_latch<R: RequestSpec<Response = protocol::Latch>>() {}
/// answers_a_latch::<protocol::OpenRequest>();
/// ```
///
/// # What refuses
///
/// Nothing on this trait but [`resolved`](Self::resolved), which answers
/// [`UnresolvedName`](crate::ErrorKind::UnresolvedName) when no enabled mod
/// declares [`NAME`](Self::NAME). [`respond`](Self::respond) meets that same
/// refusal and has no caller to hand it to, so it writes it; its own page says
/// what that costs. The refusals a caller meets belong to
/// [`client::request`](crate::client::request), whose page lists them, and the
/// answering handler's own no is one of them.
pub trait RequestSpec: Message + Default {
    /// What the answer carries.
    type Response: Message + Default;

    /// The declared name: the `rpc`'s name in the manifest charset, so
    /// `rpc Open` declares `open`.
    const NAME: &'static str;

    /// The session's number for this name, resolved on first use and kept, for
    /// the reason [`SignalSpec::resolved`](super::SignalSpec::resolved) gives.
    fn resolved() -> Result<RequestId> {
        resolved::request(Self::NAME)
    }

    /// Answers this request: the handler runs whenever a client half asks the
    /// name this type declares.
    ///
    /// The registration opens with the request type, so the line names the
    /// question and nothing else has to. `handler` is given the event, the
    /// [`Player`](crate::server::Player) whose client half asked, and the
    /// request already decoded into `Self`. It answers
    /// [`Response`](Self::Response) or a [`Refusal`](crate::Refusal), and both
    /// arms ride the same wire back, so a no reaches the caller as an
    /// [`Error`](crate::Error) of kind [`Refused`](crate::ErrorKind::Refused)
    /// carrying the handler's own words rather than as silence.
    ///
    /// This is the one handler in a server half with a caller waiting on it,
    /// and therefore the one place a [`Result`](crate::Result) is an ANSWER
    /// rather than a report. A closure needs no type annotations, because
    /// `Self` settles the third argument and [`Response`](Self::Response) the
    /// success arm:
    ///
    /// ```
    /// use ironlark::server::prelude::*;
    /// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
    /// // A real crate writes `mod protocol;` here. The hidden line is what the
    /// // generator wrote for
    /// //
    /// //     service Server { rpc Open(OpenRequest) returns (Latch); }
    /// //     message OpenRequest { uint32 force = 1; }
    /// //
    /// // in the mod's protocol.proto. `ServerMod`, `Context`, `Player`,
    /// // `Refusal`, `Result` and `RequestSpec` are the prelude's.
    /// use protocol::{Latch, OpenRequest};
    ///
    /// struct Door;
    ///
    /// impl ServerMod for Door {
    ///     async fn init() {
    ///         // A named handler, which is what a half of any size writes.
    ///         OpenRequest::respond(open);
    ///     }
    /// }
    ///
    /// async fn open(_ctx: Context, _caller: Player, ask: OpenRequest) -> Result<Latch, Refusal> {
    ///     if ask.force > 10 {
    ///         return Err(Refusal::out_of_range("force", 0..=10));
    ///     }
    ///     Ok(Latch { open: true })
    /// }
    ///
    /// // The same registration as a closure: `ask` is an `OpenRequest` and the
    /// // answer is a `Latch` because `OpenRequest::respond` says so.
    /// OpenRequest::respond(|_ctx, _caller, ask| async move {
    ///     Ok(Latch { open: ask.force > 0 })
    /// });
    /// ```
    ///
    /// A request is offered to the mod that declared it and to no other, so
    /// answering says nothing about anybody else's `open`.
    ///
    /// # When to register, and what a second one does
    ///
    /// [`init`](crate::server::ServerMod::init) is the usual place, and it is
    /// not the only legal one: a handler may be registered at any moment the
    /// half is running. A name asked before anything answers it refuses the
    /// caller, which is the same answer a session gives for a half that never
    /// registered at all.
    ///
    /// Registering the same request again REPLACES the handler and writes a
    /// `debug` line saying so, for the reason
    /// [`SignalSpec::observe`](super::SignalSpec::observe) gives.
    ///
    /// # What refuses
    ///
    /// Nothing here, because registering answers no [`Result`](crate::Result).
    /// A name no enabled mod declares is an `error` line naming the request,
    /// written at this call; the handler is not installed and the request goes
    /// on being refused.
    ///
    /// The handler itself refuses in two ways. A payload that will not decode
    /// as `Self` is turned into a refusal before the handler is entered, so a
    /// handler never sees a half-parsed value. Everything after that is the
    /// handler's own no, written as a [`Refusal`](crate::Refusal) case naming
    /// the rule the caller broke.
    fn respond<F, Fut>(handler: F)
    where
        F: Fn(Context, Player, Self) -> Fut + 'static,
        Fut: Future<Output = Result<Self::Response, Refusal>> + 'static,
    {
        crate::host::dispatch::respond::<Self, F, Fut>(handler);
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) async fn request<R: RequestSpec>(req: &R) -> Result<R::Response> {
    let id = R::resolved()?;
    let bytes = encode(req);
    match crate::bindings::client::ironlark::host::request::request(id.to_wire(), bytes).await {
        Ok(answer) => decode(&answer),
        Err(e) => Err(Error::from_wire(e.code, e.message, e.data)),
    }
}

/// Off wasm the answer comes from what a test stated, and otherwise from this
/// mod's own registered handler, so a client half's logic is testable against
/// the server half it ships with.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn request<R: RequestSpec>(req: &R) -> Result<R::Response> {
    let name = R::NAME;
    let id = R::resolved()?;
    let bytes = encode(req);
    crate::testing::request(name, &bytes)?;
    if let Some(stated) = crate::testing::configured_response(name) {
        return decode(&stated);
    }
    let caller = crate::testing::FakePlayer::new(crate::ids::SessionId::new(0)).into_player();
    match crate::host::dispatch::dispatch_request(crate::context::detached(), caller, id, bytes)
        .await
    {
        Ok(answer) => decode(&answer),
        Err(refusal) => Err(Error::from_wire(
            crate::error::code::REFUSAL,
            refusal.to_string(),
            Vec::new(),
        )),
    }
}
