use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken, token_interface::{CloseAccount, close_account, Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked}
};

use crate::{state::offer::Offer, ANCHOR_DISCRIMINATOR, transfer_tokens};


#[derive(Accounts)]
#[instruction(id:u64)]
pub struct MakeOffer<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub token_mint_a: InterfaceAccount<'info, Mint>,

    #[account(mint::token_program = token_program)]
    pub token_mint_b: InterfaceAccount<'info, Mint>,

    #[account(
        mut, 
        associated_token::mint = token_mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_token_account_a: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = maker,
        space = ANCHOR_DISCRIMINATOR + Offer::INIT_SPACE,
        seeds = [b"offer", maker.key().as_ref(), id.to_le_bytes().as_ref()],
        bump
    )]
    pub offer: Account<'info, Offer>,

    #[account(
        init,
        payer = maker,
        associated_token::mint = token_mint_a,
        associated_token::authority = offer,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>, 
}



pub fn send_offered_tokens_to_vault(ctx: &Context<MakeOffer>, token_a_amount_in: &u64) -> Result<()> {
    transfer_tokens(
        &ctx.accounts.maker_token_account_a, 
        &ctx.accounts.vault, 
        token_a_amount_in, 
        &ctx.accounts.token_mint_a, 
        &ctx.accounts.maker, 
        &ctx.accounts.token_program,
    )
}


pub fn save_offer(ctx: Context<MakeOffer>, id: &u64, token_b_wanted_amount: &u64) -> Result<()> {
    ctx.accounts.offer.set_inner(Offer {
       id: *id,
       maker: ctx.accounts.maker.key(),
       token_mint_a: ctx.accounts.token_mint_a.key(),
       token_mint_b: ctx.accounts.token_mint_b.key(),
       token_b_wanted: *token_b_wanted_amount,
       bump: ctx.bumps.offer 
    });
    Ok(())
}


#[derive(Accounts)]
pub struct TakeOffer<'info> {
    #[account(mut)] // balance will change (paying gas)
    pub taker: Signer<'info>,

    pub maker: SystemAccount<'info>,

    pub token_mint_a: InterfaceAccount<'info, Mint>,
    
    pub token_mint_b: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = token_mint_a,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_token_a_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut, // balance will change
        associated_token::mint = token_mint_b,
        associated_token::authority = taker, // taker must be owner of taker_token_b_account
        associated_token::token_program = token_program,
    )]
    pub taker_token_b_account: Box<InterfaceAccount<'info, TokenAccount>>,
    
     #[account(
        init_if_needed, // maker may not have a token b account
        payer = taker,
        associated_token::mint = token_mint_b,
        associated_token::authority = maker, // maker owns their account
        associated_token::token_program = token_program,
    )]
    pub maker_token_b_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        close = maker, // maker payed for the account so makes sense to send funds back to them when closing 
        has_one = maker,
        has_one = token_mint_a,
        has_one = token_mint_b,
        seeds = [b"offer", maker.key().as_ref(), offer.id.to_le_bytes().as_ref()],
        bump
    )]
    pub offer: Account<'info, Offer>,

    #[account(
        mut,
        associated_token::mint = token_mint_a,
        associated_token::authority = offer,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>, 
}


pub fn send_tokens_to_maker(ctx: &Context<TakeOffer>) -> Result<()> {
    transfer_tokens(
        &ctx.accounts.taker_token_b_account,
        &ctx.accounts.maker_token_b_account,
        &ctx.accounts.offer.token_b_wanted,
        &ctx.accounts.token_mint_b,
        &ctx.accounts.taker,
        &ctx.accounts.token_program,
    )
}


pub fn pull_tokens_from_vault(ctx: &Context<TakeOffer>) -> Result<()> {
    let seeds = &[
        b"offer",
        ctx.accounts.maker.to_account_info().key.as_ref(),
        &ctx.accounts.offer.id.to_le_bytes()[..],
        &[ctx.accounts.offer.bump]
    ];
    let signer_seeds = [&seeds[..]];

    let accounts = TransferChecked {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.taker_token_a_account.to_account_info(),
        mint: ctx.accounts.token_mint_a.to_account_info(),
        authority: ctx.accounts.offer.to_account_info(),
    };

    // allows to sign this tx as the offer account (it owns the vault)
    let cpi_ctx = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), accounts, &signer_seeds);

    transfer_checked(
        cpi_ctx,
        ctx.accounts.offer.token_b_wanted,
        ctx.accounts.token_mint_b.decimals,
    )?;

    let accounts = CloseAccount {
        account: ctx.accounts.vault.to_account_info(),
        destination: ctx.accounts.maker.to_account_info(),
        authority: ctx.accounts.offer.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), accounts, &signer_seeds);

    close_account(cpi_ctx)

}