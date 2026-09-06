#![allow(dead_code)]

use anchor_lang::{InstructionData, ToAccountMetas};
use litesvm::{types::FailedTransactionMetadata, LiteSVM};
use solana_keypair::Keypair;
use solana_message::{AccountMeta, Instruction, Message};
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::{fs, path::PathBuf};

pub const DECIMALS: u8 = 6;
pub const AIRDROP_LAMPORTS: u64 = 2_000_000_000;

pub fn token_program() -> anchor_lang::prelude::Pubkey {
    anchor_spl::token_2022::ID
}

pub fn associated_token_program() -> anchor_lang::prelude::Pubkey {
    anchor_spl::associated_token::ID
}

pub fn program_bytes() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/deploy/solana_level_1_token_starter.so");
    fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "Build the program with `anchor build --ignore-keys` before running tests. Could not read {}: {error}",
            path.display()
        )
    })
}

pub fn setup() -> (LiteSVM, Keypair) {
    let program_id = solana_level_1_token_starter::ID;
    let mut svm = LiteSVM::new();
    svm.add_program(program_id, &program_bytes())
        .expect("program must load");

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), AIRDROP_LAMPORTS)
        .expect("airdrop must succeed");
    (svm, payer)
}

pub fn associated_token_address(
    owner: anchor_lang::prelude::Pubkey,
    mint: anchor_lang::prelude::Pubkey,
) -> anchor_lang::prelude::Pubkey {
    anchor_lang::prelude::Pubkey::find_program_address(
        &[owner.as_ref(), token_program().as_ref(), mint.as_ref()],
        &associated_token_program(),
    )
    .0
}

pub fn send_instruction(
    svm: &mut LiteSVM,
    payer: &Keypair,
    signers: &[&Keypair],
    instruction: Instruction,
) -> Result<(), FailedTransactionMetadata> {
    let blockhash = svm.latest_blockhash();
    let message = Message::new(&[instruction], Some(&payer.pubkey()));
    let transaction = Transaction::new(signers, message, blockhash);
    svm.send_transaction(transaction).map(|_| ())
}

pub fn assert_tx_err(result: Result<(), FailedTransactionMetadata>, context: &str) {
    let error = result.expect_err(context);
    assert!(
        !error.meta.logs.is_empty(),
        "{context}: failed transaction must emit program logs"
    );
}

pub fn logs_contain(error: &FailedTransactionMetadata, needle: &str) -> bool {
    error.meta.logs.iter().any(|log| log.contains(needle))
}

fn account_metas(accounts: impl ToAccountMetas) -> Vec<AccountMeta> {
    accounts
        .to_account_metas(None)
        .into_iter()
        .map(|meta| AccountMeta {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        })
        .collect()
}

pub fn create_token_ix(
    payer: &Keypair,
    authority: &Keypair,
    mint: &Keypair,
    decimals: u8,
) -> Instruction {
    let accounts = solana_level_1_token_starter::accounts::CreateToken {
        payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint: mint.pubkey(),
        token_program: token_program(),
        system_program: anchor_lang::system_program::ID,
    };
    Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: account_metas(accounts),
        data: solana_level_1_token_starter::instruction::CreateToken { decimals }.data(),
    }
}

pub fn create_token(
    svm: &mut LiteSVM,
    payer: &Keypair,
    authority: &Keypair,
    decimals: u8,
) -> Keypair {
    let mint = Keypair::new();
    send_instruction(
        svm,
        payer,
        &[payer, authority, &mint],
        create_token_ix(payer, authority, &mint, decimals),
    )
    .expect("create_token must succeed");
    mint
}

pub fn create_token_account_ix(
    payer: &Keypair,
    owner: anchor_lang::prelude::Pubkey,
    mint: anchor_lang::prelude::Pubkey,
) -> Instruction {
    let token_account = associated_token_address(owner, mint);
    let accounts = solana_level_1_token_starter::accounts::CreateTokenAccount {
        payer: payer.pubkey(),
        owner,
        mint,
        token_account,
        token_program: token_program(),
        associated_token_program: associated_token_program(),
        system_program: anchor_lang::system_program::ID,
    };
    Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: account_metas(accounts),
        data: solana_level_1_token_starter::instruction::CreateTokenAccount {}.data(),
    }
}

pub fn create_token_account(
    svm: &mut LiteSVM,
    payer: &Keypair,
    owner: anchor_lang::prelude::Pubkey,
    mint: anchor_lang::prelude::Pubkey,
) -> anchor_lang::prelude::Pubkey {
    let token_account = associated_token_address(owner, mint);
    send_instruction(
        svm,
        payer,
        &[payer],
        create_token_account_ix(payer, owner, mint),
    )
    .expect("create_token_account must succeed");
    token_account
}

pub fn mint_tokens_ix(
    authority: &Keypair,
    mint: anchor_lang::prelude::Pubkey,
    destination: anchor_lang::prelude::Pubkey,
    amount: u64,
) -> Instruction {
    let accounts = solana_level_1_token_starter::accounts::MintTokens {
        authority: authority.pubkey(),
        mint,
        destination,
        token_program: token_program(),
    };
    Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: account_metas(accounts),
        data: solana_level_1_token_starter::instruction::MintTokens { amount }.data(),
    }
}

pub fn mint_tokens(
    svm: &mut LiteSVM,
    payer: &Keypair,
    authority: &Keypair,
    mint: anchor_lang::prelude::Pubkey,
    destination: anchor_lang::prelude::Pubkey,
    amount: u64,
) {
    send_instruction(
        svm,
        payer,
        &[payer, authority],
        mint_tokens_ix(authority, mint, destination, amount),
    )
    .expect("mint_tokens must succeed");
}

pub fn transfer_tokens_ix(
    authority: &Keypair,
    mint: anchor_lang::prelude::Pubkey,
    source: anchor_lang::prelude::Pubkey,
    destination: anchor_lang::prelude::Pubkey,
    amount: u64,
) -> Instruction {
    let accounts = solana_level_1_token_starter::accounts::TransferTokens {
        authority: authority.pubkey(),
        mint,
        source,
        destination,
        token_program: token_program(),
    };
    Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: account_metas(accounts),
        data: solana_level_1_token_starter::instruction::TransferTokens { amount }.data(),
    }
}

pub fn transfer_tokens(
    svm: &mut LiteSVM,
    payer: &Keypair,
    authority: &Keypair,
    mint: anchor_lang::prelude::Pubkey,
    source: anchor_lang::prelude::Pubkey,
    destination: anchor_lang::prelude::Pubkey,
    amount: u64,
) {
    send_instruction(
        svm,
        payer,
        &[payer, authority],
        transfer_tokens_ix(authority, mint, source, destination, amount),
    )
    .expect("transfer_tokens must succeed");
}

/// SPL Token / Token-2022 mint layout (first 82 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintState {
    pub mint_authority: Option<anchor_lang::prelude::Pubkey>,
    pub supply: u64,
    pub decimals: u8,
}

pub fn unpack_mint(data: &[u8]) -> MintState {
    assert!(
        data.len() >= 82,
        "mint account must contain the 82-byte base mint layout"
    );
    MintState {
        mint_authority: unpack_coption_pubkey(&data[0..36]),
        supply: u64::from_le_bytes(data[36..44].try_into().expect("supply bytes")),
        decimals: data[44],
    }
}

/// SPL Token / Token-2022 token-account layout (first 72 bytes of the base account).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenAccountState {
    pub mint: anchor_lang::prelude::Pubkey,
    pub owner: anchor_lang::prelude::Pubkey,
    pub amount: u64,
}

pub fn unpack_token_account(data: &[u8]) -> TokenAccountState {
    assert!(
        data.len() >= 72,
        "token account must contain mint, owner and amount"
    );
    TokenAccountState {
        mint: anchor_lang::prelude::Pubkey::new_from_array(
            data[0..32].try_into().expect("mint bytes"),
        ),
        owner: anchor_lang::prelude::Pubkey::new_from_array(
            data[32..64].try_into().expect("owner bytes"),
        ),
        amount: u64::from_le_bytes(data[64..72].try_into().expect("amount bytes")),
    }
}

pub fn read_mint(svm: &LiteSVM, mint: &anchor_lang::prelude::Pubkey) -> MintState {
    let account = svm.get_account(mint).expect("mint must exist");
    assert_eq!(
        account.owner,
        token_program(),
        "mint must be owned by Token-2022"
    );
    unpack_mint(&account.data)
}

pub fn read_token_account(
    svm: &LiteSVM,
    token_account: &anchor_lang::prelude::Pubkey,
) -> TokenAccountState {
    let account = svm.get_account(token_account).expect("token account must exist");
    assert_eq!(
        account.owner,
        token_program(),
        "token account must be owned by Token-2022"
    );
    unpack_token_account(&account.data)
}

fn unpack_coption_pubkey(data: &[u8]) -> Option<anchor_lang::prelude::Pubkey> {
    let tag = u32::from_le_bytes(data[0..4].try_into().expect("coption tag"));
    match tag {
        0 => None,
        1 => Some(anchor_lang::prelude::Pubkey::new_from_array(
            data[4..36].try_into().expect("coption pubkey"),
        )),
        other => panic!("invalid COption tag: {other}"),
    }
}
