use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

pub struct Escrow {
    // 1 byte
    pub is_initialized: bool,
    // 32 bytes
    pub initializer_pubkey: Pubkey,
    // This will be the taker's pubkey
    // 32 bytes
    pub temp_token_account_pubkey: Pubkey,
    // This is the account "taker" will send their tokens to
    // 32 bytes
    pub initializer_token_to_receive_account_pubkey: Pubkey,
    // This is to check the taker has sent enough
    // 8 bytes
    pub expected_amount: u64,
}

// This is Solona's versin of the Sized trait
impl Sealed for Escrow {}

impl IsInitialized for Escrow {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for Escrow {
    const LEN: usize = 105;

    // This bascially unpacks the input bytes into the data structure
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        // Generates an array reference to slicable data
        let src = array_ref![src, 0, Escrow::LEN];
        // You can use `array_refs` to generate a series of array references
        // to an input array reference.  The idea is if you want to break an
        // array into a series of contiguous and non-overlapping arrays.
        let (
            is_initialized,
            initializer_pubkey,
            temp_token_account_pubkey,
            initializer_token_to_receive_account_pubkey,
            expected_amount,
        ) = array_refs![src, 1, 32, 32, 32, 8];

        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => {
                return Err(ProgramError::InvalidAccountData);
            }
        };

        Ok(Escrow {
            is_initialized,
            initializer_pubkey: Pubkey::new_from_array(*initializer_pubkey),
            temp_token_account_pubkey: Pubkey::new_from_array(*temp_token_account_pubkey),
            initializer_token_to_receive_account_pubkey: Pubkey::new_from_array(
                *initializer_token_to_receive_account_pubkey,
            ),
            expected_amount: u64::from_le_bytes(*expected_amount),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        // we are creating an array ref from a slicable data
        let dst = array_mut_ref![dst, 0, Escrow::LEN];
        let (
            is_initialized,
            initializer_pubkey,
            temp_token_account_pubkey,
            initializer_token_to_receive_account_pubkey,
            expected_amount,
        ) = mut_array_refs![dst, 1, 32, 32, 32, 8];

        is_initialized[0] = self.is_initialized as u8;
        initializer_pubkey.copy_from_slice(self.initializer_pubkey.as_ref());
        temp_token_account_pubkey.copy_from_slice(self.temp_token_account_pubkey.as_ref());
        initializer_token_to_receive_account_pubkey
            .copy_from_slice(self.initializer_token_to_receive_account_pubkey.as_ref());
        expected_amount.copy_from_slice(&self.expected_amount.to_le_bytes());
    }
}
