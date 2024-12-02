pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("2evh2Y7yAMpWMu1DoNLPLgXQHMBUsmna4zmj7kaGMjNP");

#[program]
pub mod escrow_swap {
    use super::*;

    pub fn make_offer(
        ctx: Context<MakeOffer>,
        id: u64,
        token_a_amount_in: u64,
        token_b_amount_wanted: u64,
    ) -> Result<()> {
        instructions::offer::send_offered_tokens_to_vault(&ctx, &token_a_amount_in)?;
        instructions::offer::save_offer(ctx, &id, &token_b_amount_wanted)
    }

    pub fn take_offer(ctx: Context<TakeOffer>) -> Result<()> {
        instructions::offer::send_tokens_to_maker(&ctx)?;
        instructions::offer::pull_tokens_from_vault(&ctx)
    }
}
