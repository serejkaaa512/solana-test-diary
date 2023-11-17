use anchor_lang::prelude::borsh::{BorshDeserialize, BorshSerialize};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::PUBKEY_BYTES;
use solana_program::program_pack::{Pack, Sealed};

declare_id!("bNFMSsTXGZxhAA7mUcdUid5Yir3zWJf1myfP4TSQ46x");

// Задание
// Написать контракт на анкоре, который реализует систему он-чейн дневника
// Программа должна позволять создавать "дневник" для пользователя, создавать записи в этом "дневнике".
// Запись должна быть условно не ограничена по размерам (+-максимальный размер аккаунта в солане)
// У одного пользователя может быть несколько дневников, в рамках одного дневника может быть 0 и больше записей.

//  Max PDA size = 10240 bytes => MAX RECORDS < ((10240 - 20 - 1 - 4 - 8) / 32  = 318)
pub const MAX_RECORDS: usize = 318;

pub const MAX_NAME_LENGHT: usize = 20;

pub const DIARY_LEN: usize = 4       // id
+ 4                                  // records_len
+ PUBKEY_BYTES * MAX_RECORDS         // records
+ MAX_NAME_LENGHT                    // name
+ 1                                  // bump
;

pub const RECORD_LEN: usize = 10_000_000 // MAX ACCOUNT LEN +/-
;

#[program]
mod diary {
    use super::*;
    pub fn create_diary(ctx: Context<CreateDiary>, id: u32, name: String) -> Result<()> {
        require!(name.len() < MAX_NAME_LENGHT, CustomError::NameIsTooLong);
        ctx.accounts.diary_account.records = vec![];
        ctx.accounts.diary_account.id = id;
        ctx.accounts.diary_account.name = name;
        ctx.accounts.diary_account.bump = *ctx.bumps.get("diary_account").unwrap();
        Ok(())
    }
    pub fn add_record(ctx: Context<AddRecord>, _id: u32, text: String, offset: u32) -> Result<()> {
        let offset = offset as usize;
        let new_bytes_len_from_offset = offset + text.len();
        if ctx
            .accounts
            .diary_account
            .records
            .contains(&ctx.accounts.records_account.key())
        {
            let mut record_data =
                Record::unpack_from_slice(&*ctx.accounts.records_account.data.borrow())
                    .expect("deposit token unpack");
            let mut bytes = record_data.text.into_bytes();
            let new_bytes = if bytes.len() < new_bytes_len_from_offset {
                let mut new_bytes = vec![0; new_bytes_len_from_offset];
                new_bytes[..bytes.len()].copy_from_slice(&bytes);
                new_bytes[offset..].copy_from_slice(&text.as_bytes());
                new_bytes
            } else {
                bytes[offset..new_bytes_len_from_offset].copy_from_slice(&text.as_bytes());
                bytes
            };

            record_data = Record {
                text: String::from_utf8(new_bytes).unwrap(),
            };

            Record::pack(
                record_data,
                &mut ctx.accounts.records_account.data.borrow_mut(),
            )?;
        } else {
            ctx.accounts
                .diary_account
                .records
                .push(ctx.accounts.records_account.key());

            let mut new_bytes = vec![0; new_bytes_len_from_offset];
            new_bytes[offset..].copy_from_slice(&text.as_bytes());

            let record_account_data = Record {
                text: String::from_utf8(new_bytes).unwrap(),
            };

            Record::pack(
                record_account_data,
                &mut ctx.accounts.records_account.data.borrow_mut(),
            )?;
        }
        Ok(())
    }

    pub fn remove_record(ctx: Context<RemoveRecord>, _id: u32) -> Result<()> {
        if let Some(index) = ctx
            .accounts
            .diary_account
            .records
            .iter()
            .position(|x| *x == ctx.accounts.records_account.key())
        {
            ctx.accounts.diary_account.records.remove(index);

            // close an account (erase all stored data) by removing all SOL
            let dest_starting_lamports = ctx.accounts.authority.lamports();
            **ctx.accounts.authority.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(ctx.accounts.records_account.lamports())
                .unwrap();
            **ctx.accounts.records_account.lamports.borrow_mut() = 0;

            let mut source_data = ctx.accounts.records_account.data.borrow_mut();
            source_data.fill(0);
        }
        Ok(())
    }
}

#[account]
#[derive(Default)]
pub struct Diary {
    pub id: u32,
    pub name: String,
    pub records: Vec<Pubkey>,
    pub bump: u8,
}

#[derive(Accounts)]
#[instruction(id: u32)]
pub struct CreateDiary<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(init, payer = authority,
    space = 8 + DIARY_LEN,
    seeds = [&authority.key().to_bytes(), "diary".as_bytes(), id.to_string().as_bytes()], bump)]
    pub diary_account: Account<'info, Diary>,
    pub system_program: Program<'info, System>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct Record {
    pub text: String,
}

impl Sealed for Record {}

impl Pack for Record {
    const LEN: usize = RECORD_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let data = self.try_to_vec().unwrap();
        let (left, _) = dst.split_at_mut(data.len());
        left.copy_from_slice(&data);
    }

    fn unpack_from_slice(
        mut src: &[u8],
    ) -> std::result::Result<Self, anchor_lang::prelude::ProgramError> {
        let unpacked = Self::deserialize(&mut src)?;
        Ok(unpacked)
    }
}

#[derive(Accounts)]
#[instruction(id: u32)]
pub struct AddRecord<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut,
    seeds = [&authority.key().to_bytes(), "diary".as_bytes(), id.to_string().as_bytes()], bump = diary_account.bump)]
    pub diary_account: Account<'info, Diary>,
    #[account(mut)]
    pub records_account: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(id: u32)]
pub struct RemoveRecord<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut,
    seeds = [&authority.key().to_bytes(), "diary".as_bytes(), id.to_string().as_bytes()], bump = diary_account.bump)]
    pub diary_account: Account<'info, Diary>,
    #[account(mut)]
    pub records_account: Signer<'info>,
}

#[error_code]
pub enum CustomError {
    NameIsTooLong,
}
