use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};

use crate::{error::EscrowError, instructions::EscrowInstruction, state::Escrow};

pub struct Processor;
impl Processor {
    // This is directly used by the entrypoint and the accounts here are from the tx
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = EscrowInstruction::unpack(instruction_data)?;

        match instruction {
            EscrowInstruction::InitEscrow { amount } => {
                msg!("Instruction: InitEscrow");
                Self::process_init_escrow(accounts, amount, program_id)
            }
        }
    }

    fn process_init_escrow(
        accounts: &[AccountInfo],
        amount: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        // Recall the accounts in instructions, the order of those accounts
        // 0. `[signer]` The account of the person initializing the escrow
        // 1. `[writable]` Temporary token account that should be created prior to this instruction and owned by the initializer
        // 2. `[]` The initializer's token account for the token they will receive should the trade go through
        // 3. `[writable]` The escrow account, it will hold all necessary info about the trade.
        // 4. []` The rent sysvar
        // 5. `[]` The token program

        let account_info_iter = &mut accounts.iter();
        // This returns the Account
        // 0 - signer (Alice who initialises the escrow)
        let initializer = next_account_info(account_info_iter)?;
        if !initializer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // 1 - temp token account where Alice moves token into and then make the escrow the
        //   authority
        let temp_token_account = next_account_info(account_info_iter)?;

        // 2 - Alice will receive her tokens from Bob here
        // Must be owned by spl_token program id's derived addr? because it needs to make the
        // authority Alice in the future
        let token_to_receive_account = next_account_info(account_info_iter)?;
        if *token_to_receive_account.owner != spl_token::id() {
            return Err(ProgramError::IncorrectProgramId);
        }

        // 3.- this is the escrow account to hold data about the escrow
        let escrow_account = next_account_info(account_info_iter)?;

        // 4 - sysvar?
        //  These sysvars can be accessed through accounts and store parameters such as what the
        //  current fee or rent is
        //  In fact, in the newer versions, we can just get the sysvar without passing in the
        //  account to get sysvar data
        let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;

        // Here we are saying we want the rent to be exempt
        // i.e. the escrow account has some minimum balance
        // TODO: udnerstand consequence
        if !rent.is_exempt(escrow_account.lamports(), escrow_account.data_len()) {
            return Err(EscrowError::NotRentExempt.into());
        }

        // Now lets make sure that the escrow_account is empty and not initialised
        // We do this by checking the account data
        //
        // Escrow::unpack_unchecked is auto implement with Escrow::unpack_from_slice
        // that we implemented
        let mut escrow_info = Escrow::unpack_unchecked(&escrow_account.try_borrow_data()?)?;
        if escrow_info.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        // Now we can initialize it
        escrow_info.is_initialized = true;
        escrow_info.initializer_pubkey = *initializer.key;
        escrow_info.temp_token_account_pubkey = *temp_token_account.key;
        escrow_info.initializer_token_to_receive_account_pubkey = *token_to_receive_account.key;
        escrow_info.expected_amount = amount;

        // This packs the new initialized data back into the data field of escrow_account
        Escrow::pack(escrow_info, &mut escrow_account.try_borrow_mut_data()?)?;

        // ----- Now we want to move the ownership of temp_token_account to the escrow account ----
        //
        // Step 1: create a pda for the escrow program
        // pda is the program derived address which is NOT on the ed25519 curve
        // and therefore does NOT have a private key associated with it
        let (pda, _bump_seed) = Pubkey::find_program_address(&[b"escrow"], program_id);

        // Step 2: create the instruction to be invoked on the token program
        // 5 - token program
        let token_program = next_account_info(account_info_iter)?;
        let owner_change_ix = spl_token::instruction::set_authority(
            token_program.key,
            temp_token_account.key,
            // user space new authority
            Some(&pda),
            spl_token::instruction::AuthorityType::AccountOwner,
            // the public key that owns this account
            // now it is the initializer.key because we are using the same keypair as
            // the authority of this temp_token_account
            initializer.key,
            &[initializer.key],
        )?;

        // Step 3: invoke the instruction
        msg!("Calling the token program to transfer token account ownership...");
        invoke(
            &owner_change_ix,
            &[
                temp_token_account.clone(),
                initializer.clone(),
                token_program.clone(),
            ],
        )?;
        Ok(())
    }
}
