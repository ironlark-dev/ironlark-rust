mod protocol {
    ironlark::protocol!("../protocol.proto");
    ironlark::declares!("../mod.toml");
}

use ironlark::server::prelude::*;
use protocol::{AdvanceRequest, Value};

/// The most one ask may move the value, so a refusal is part of what this
/// example shows.
const MOST: u32 = 1_000_000;

ironlark::state! {
    static VALUE: u32 = 0;
}

struct Echo;

#[ironlark::hooks("../mod.toml")]
impl ServerMod for Echo {
    async fn init() {
        AdvanceRequest::respond(advance);
    }

    async fn on_join(ctx: Context, player: Player) {
        log::info!("{player} joined the echo chamber on tick {}", ctx.raised_at);
    }
}

async fn advance(_ctx: Context, _caller: Player, ask: AdvanceRequest) -> Result<Value, Refusal> {
    if ask.amount > MOST {
        return Err(Refusal::out_of_range("amount", 0..=i64::from(MOST)));
    }
    let value = VALUE.get() + ask.amount;
    VALUE.set(value);
    if let Err(e) = signal(&Value { value }) {
        log::warn!("echo: the new value did not go out: {e}");
    }
    if let Err(e) = audio::play(protocol::sound::Ping, SoundBus::Effects).await {
        log::warn!("echo: the ping did not play: {e}");
    }
    Ok(Value { value })
}

ironlark::export_server!(Echo);

#[cfg(test)]
mod tests {
    use super::*;
    use ironlark::testing::{FakePlayer, block_on, context, take_plays, take_signals};
    use ironlark::{SessionId, Tick};

    fn event() -> Context {
        context(Tick::new(54), Tick::new(54))
    }

    fn caller(session: u64) -> Player {
        FakePlayer::new(SessionId::new(session)).into_player()
    }

    #[test]
    fn an_advance_moves_the_value_and_states_it() {
        let answer = block_on(advance(event(), caller(1), AdvanceRequest { amount: 7 }));
        let Ok(stated) = answer else {
            panic!("7 is inside the range");
        };
        assert_eq!(stated.value, 7);
        assert_eq!(VALUE.get(), 7);
        let raised = take_signals();
        assert_eq!(raised.len(), 1);
        assert_eq!(raised[0].name, "value");
        assert_eq!(
            take_plays().len(),
            1,
            "the declared ping plays once per advance"
        );
    }

    #[test]
    fn an_amount_over_the_cap_is_refused_with_its_numbers() {
        let answer = block_on(advance(
            event(),
            caller(2),
            AdvanceRequest { amount: 2_000_000 },
        ));
        assert!(matches!(
            answer,
            Err(Refusal::OutOfRange {
                min: 0,
                max: 1_000_000,
                ..
            })
        ));
    }
}
