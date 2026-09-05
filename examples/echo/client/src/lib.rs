mod protocol {
    ironlark::protocol!("../protocol.proto");
}

use ironlark::client::prelude::*;
use protocol::{AdvanceRequest, Value};

struct Echo;

#[ironlark::hooks("../mod.toml")]
impl ClientMod for Echo {
    async fn init() {
        Value::observe(on_value);
    }

    /// The mod's own hook. `mod.toml` binds it to a key by default and the
    /// player rebinds it.
    async fn echo(_ctx: Context, edge: InputEdge) {
        if edge != InputEdge::Pressed {
            return;
        }
        match request(&AdvanceRequest { amount: 1 }).await {
            Ok(answer) => log::info!("the server answered {}", answer.value),
            Err(e) => log::error!("echo: the server did not answer: {e}"),
        }
    }
}

async fn on_value(_ctx: Context, _from: SourceId, stated: Value) {
    log::info!("the value is now {}", stated.value);
}

ironlark::export_client!(Echo);
