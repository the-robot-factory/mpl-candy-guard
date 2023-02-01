use super::*;

use solana_program::{program::invoke, system_instruction};

use crate::{errors::CandyGuardError, state::GuardType, utils::assert_keys_equal};

const REVENUE: u64 = 1000;

/// Guard that charges an amount in SOL (lamports) for the mint.
///
/// List of accounts required:
///
///   0. `[]` Account to receive the funds.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct SolPayment {
    pub lamports: u64,
    pub destination: Pubkey,
}

impl Guard for SolPayment {
    fn size() -> usize {
        8    // lamports
        + 32 // destination
    }

    fn mask() -> u64 {
        GuardType::as_mask(GuardType::SolPayment)
    }
}

impl Condition for SolPayment {
    fn validate<'info>(
        &self,
        ctx: &Context<'_, '_, '_, 'info, Mint<'info>>,
        _mint_args: &[u8],
        _guard_set: &GuardSet,
        evaluation_context: &mut EvaluationContext,
    ) -> Result<()> {
        let index = evaluation_context.account_cursor;
        // validates that we received all required accounts
        let destination = try_get_account_info(ctx, index)?;
        evaluation_context.account_cursor += 1;
        // validates the account information
        assert_keys_equal(destination.key, &self.destination)?;

        evaluation_context
            .indices
            .insert("lamports_destination", index);

        if ctx.accounts.payer.lamports() < self.lamports {
            msg!(
                "Require {} lamports, accounts has {} lamports",
                self.lamports,
                ctx.accounts.payer.lamports(),
            );
            return err!(CandyGuardError::NotEnoughSOL);
        }

        Ok(())
    }

    fn pre_actions<'info>(
        &self,
        ctx: &Context<'_, '_, '_, 'info, Mint<'info>>,
        _mint_args: &[u8],
        _guard_set: &GuardSet,
        evaluation_context: &mut EvaluationContext,
    ) -> Result<()> {
        let destination =
            try_get_account_info(ctx, evaluation_context.indices["lamports_destination"])?;

        // tax payout
        // contract owner gets tax fee.
        let revenue_amount = (REVENUE * self.lamports) / 10000;
        invoke(
            &system_instruction::transfer(
                &ctx.accounts.payer.key(),
                &ctx.accounts.revenue_recipient.key(),
                revenue_amount,
            ),
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.revenue_recipient.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        let remaining_amount = self.lamports - revenue_amount;
        invoke(
            &system_instruction::transfer(
                &ctx.accounts.payer.key(),
                &destination.key(),
                remaining_amount,
            ),
            &[
                ctx.accounts.payer.to_account_info(),
                destination.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        Ok(())
    }
}
