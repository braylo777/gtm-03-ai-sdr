//! Stage 3 — AI SDRs & Hyper-Personalization.
//!
//! * `enrich`      — Clay-style waterfall enrichment: try providers in order per field, stop
//!   when filled, track credits spent, validate emails.
//! * `personalize` — pick the strongest hook from account signals, fill a template, and lint
//!   the draft (unresolved variables, length, spam words, one call to action).
//! * `sequence`    — build a multi-step schedule on business days in the prospect's time zone,
//!   with mailbox daily caps and stop rules.
//!
//! Nothing here sends email or calls a paid API.

mod api;
pub mod enrich;
pub mod personalize;
pub mod sequence;
pub mod util;

pub use api::dispatch;
use serde_json::Value;

pub const OPS: &[&str] = &["waterfall_enrich", "personalize_email", "build_sequence"];

pub fn handle(op: &str, i: &Value) -> Result<Value, String> {
    match op {
        "waterfall_enrich" => enrich::waterfall_op(i),
        "personalize_email" => personalize::email_op(i),
        "build_sequence" => sequence::sequence_op(i),
        _ => Err(format!("unknown op `{op}`; known: {}", OPS.join(", "))),
    }
}
