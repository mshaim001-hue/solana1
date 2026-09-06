#![allow(dead_code)]

use anchor_lang::{InstructionData, ToAccountMetas};
use litesvm::{types::FailedTransactionMetadata, LiteSVM};
use solana_keypair::Keypair;
use solana_message::{AccountMeta, Instruction, Message};
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::{fs, path::PathBuf};

pub const DECIMALS: u8 = 6;
pub const DEAL_ID: u64 = 42;
pub const DEAL_AMOUNT: u64 = 1_000_000;
pub const AIRDROP_LAMPORTS: u64 = 5_000_000_000;

pub fn token_program() -> anchor_lang::prelude::Pubkey {
    anchor_spl::token_2022::ID
}

pub fn associated_token_program() -> anchor_lang::prelude::Pubkey {
    anchor_spl::associated_token::ID
}

fn read_so(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/deploy")
        .join(name);
    fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "Build programs with `anchor build --ignore-keys` first. Could not read {}: {error}",
            path.display()
        )
    })
}

pub fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    svm.add_program(solana_level_1_token_starter::ID, &read_so("solana_level_1_token_starter.so"))
        .expect("token starter must load");
    svm.add_program(escrow::ID, &read_so("escrow.so"))
        .expect("escrow must load");

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

pub fn escrow_pda(
    sender: anchor_lang::prelude::Pubkey,
    deal_id: u64,
) -> (anchor_lang::prelude::Pubkey, u8) {
    anchor_lang::prelude::Pubkey::find_program_address(
        &[
            escrow::EscrowState::SEED_PREFIX,
            sender.as_ref(),
            &deal_id.to_le_bytes(),
        ],
        &escrow::ID,
    )
}

pub fn vault_address_for_mint(
    sender: anchor_lang::prelude::Pubkey,
    deal_id: u64,
    mint: anchor_lang::prelude::Pubkey,
) -> anchor_lang::prelude::Pubkey {
    associated_token_address(escrow_pda(sender, deal_id).0, mint)
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
    result.expect_err(context);
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

pub fn create_token(
    svm: &mut LiteSVM,
    payer: &Keypair,
    authority: &Keypair,
    decimals: u8,
) -> Keypair {
    let mint = Keypair::new();
    let accounts = solana_level_1_token_starter::accounts::CreateToken {
        payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint: mint.pubkey(),
        token_program: token_program(),
        system_program: anchor_lang::system_program::ID,
    };
    send_instruction(
        svm,
        payer,
        &[payer, authority, &mint],
        Instruction {
            program_id: solana_level_1_token_starter::ID,
            accounts: account_metas(accounts),
            data: solana_level_1_token_starter::instruction::CreateToken { decimals }.data(),
        },
    )
    .expect("create_token must succeed");
    mint
}

pub fn create_token_account(
    svm: &mut LiteSVM,
    payer: &Keypair,
    owner: anchor_lang::prelude::Pubkey,
    mint: anchor_lang::prelude::Pubkey,
) -> anchor_lang::prelude::Pubkey {
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
    send_instruction(
        svm,
        payer,
        &[payer],
        Instruction {
            program_id: solana_level_1_token_starter::ID,
            accounts: account_metas(accounts),
            data: solana_level_1_token_starter::instruction::CreateTokenAccount {}.data(),
        },
    )
    .expect("create_token_account must succeed");
    token_account
}

pub fn mint_tokens(
    svm: &mut LiteSVM,
    payer: &Keypair,
    authority: &Keypair,
    mint: anchor_lang::prelude::Pubkey,
    destination: anchor_lang::prelude::Pubkey,
    amount: u64,
) {
    let accounts = solana_level_1_token_starter::accounts::MintTokens {
        authority: authority.pubkey(),
        mint,
        destination,
        token_program: token_program(),
    };
    send_instruction(
        svm,
        payer,
        &[payer, authority],
        Instruction {
            program_id: solana_level_1_token_starter::ID,
            accounts: account_metas(accounts),
            data: solana_level_1_token_starter::instruction::MintTokens { amount }.data(),
        },
    )
    .expect("mint_tokens must succeed");
}

pub fn initialize_ix(
    sender: &Keypair,
    receiver: anchor_lang::prelude::Pubkey,
    mint: anchor_lang::prelude::Pubkey,
    deal_id: u64,
    amount: u64,
) -> Instruction {
    let escrow = escrow_pda(sender.pubkey(), deal_id).0;
    let vault = vault_address_for_mint(sender.pubkey(), deal_id, mint);
    let accounts = escrow::accounts::Initialize {
        sender: sender.pubkey(),
        receiver,
        mint,
        escrow,
        vault,
        token_program: token_program(),
        associated_token_program: associated_token_program(),
        system_program: anchor_lang::system_program::ID,
    };
    Instruction {
        program_id: escrow::ID,
        accounts: account_metas(accounts),
        data: escrow::instruction::Initialize { deal_id, amount }.data(),
    }
}

pub fn initialize(
    svm: &mut LiteSVM,
    payer: &Keypair,
    sender: &Keypair,
    receiver: anchor_lang::prelude::Pubkey,
    mint: anchor_lang::prelude::Pubkey,
    deal_id: u64,
    amount: u64,
) {
    send_instruction(
        svm,
        payer,
        &[payer, sender],
        initialize_ix(sender, receiver, mint, deal_id, amount),
    )
    .expect("initialize must succeed");
}

pub fn deposit_ix(
    sender: &Keypair,
    mint: anchor_lang::prelude::Pubkey,
    sender_token_account: anchor_lang::prelude::Pubkey,
    deal_id: u64,
) -> Instruction {
    let escrow = escrow_pda(sender.pubkey(), deal_id).0;
    let vault = vault_address_for_mint(sender.pubkey(), deal_id, mint);
    let accounts = escrow::accounts::Deposit {
        sender: sender.pubkey(),
        escrow,
        mint,
        sender_token_account,
        vault,
        token_program: token_program(),
    };
    Instruction {
        program_id: escrow::ID,
        accounts: account_metas(accounts),
        data: escrow::instruction::Deposit { deal_id }.data(),
    }
}

pub fn deposit(
    svm: &mut LiteSVM,
    payer: &Keypair,
    sender: &Keypair,
    mint: anchor_lang::prelude::Pubkey,
    sender_token_account: anchor_lang::prelude::Pubkey,
    deal_id: u64,
) {
    send_instruction(
        svm,
        payer,
        &[payer, sender],
        deposit_ix(sender, mint, sender_token_account, deal_id),
    )
    .expect("deposit must succeed");
}

pub fn release_ix(
    sender: &Keypair,
    receiver: anchor_lang::prelude::Pubkey,
    mint: anchor_lang::prelude::Pubkey,
    receiver_token_account: anchor_lang::prelude::Pubkey,
    deal_id: u64,
) -> Instruction {
    let escrow = escrow_pda(sender.pubkey(), deal_id).0;
    let vault = vault_address_for_mint(sender.pubkey(), deal_id, mint);
    let accounts = escrow::accounts::Release {
        sender: sender.pubkey(),
        receiver,
        escrow,
        mint,
        vault,
        receiver_token_account,
        token_program: token_program(),
    };
    Instruction {
        program_id: escrow::ID,
        accounts: account_metas(accounts),
        data: escrow::instruction::Release { deal_id }.data(),
    }
}

pub fn release(
    svm: &mut LiteSVM,
    payer: &Keypair,
    sender: &Keypair,
    receiver: anchor_lang::prelude::Pubkey,
    mint: anchor_lang::prelude::Pubkey,
    receiver_token_account: anchor_lang::prelude::Pubkey,
    deal_id: u64,
) {
    send_instruction(
        svm,
        payer,
        &[payer, sender],
        release_ix(sender, receiver, mint, receiver_token_account, deal_id),
    )
    .expect("release must succeed");
}

pub fn cancel_ix(
    sender: &Keypair,
    mint: anchor_lang::prelude::Pubkey,
    sender_token_account: anchor_lang::prelude::Pubkey,
    deal_id: u64,
) -> Instruction {
    let escrow = escrow_pda(sender.pubkey(), deal_id).0;
    let vault = vault_address_for_mint(sender.pubkey(), deal_id, mint);
    let accounts = escrow::accounts::Cancel {
        sender: sender.pubkey(),
        escrow,
        mint,
        sender_token_account,
        vault,
        token_program: token_program(),
    };
    Instruction {
        program_id: escrow::ID,
        accounts: account_metas(accounts),
        data: escrow::instruction::Cancel { deal_id }.data(),
    }
}

pub fn cancel(
    svm: &mut LiteSVM,
    payer: &Keypair,
    sender: &Keypair,
    mint: anchor_lang::prelude::Pubkey,
    sender_token_account: anchor_lang::prelude::Pubkey,
    deal_id: u64,
) {
    send_instruction(
        svm,
        payer,
        &[payer, sender],
        cancel_ix(sender, mint, sender_token_account, deal_id),
    )
    .expect("cancel must succeed");
}

pub fn unpack_token_amount(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[64..72].try_into().expect("amount bytes"))
}

pub fn unpack_mint_supply(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[36..44].try_into().expect("supply bytes"))
}

pub fn token_amount(svm: &LiteSVM, token_account: &anchor_lang::prelude::Pubkey) -> u64 {
    let account = svm.get_account(token_account).expect("token account must exist");
    unpack_token_amount(&account.data)
}

pub fn mint_supply(svm: &LiteSVM, mint: &anchor_lang::prelude::Pubkey) -> u64 {
    let account = svm.get_account(mint).expect("mint must exist");
    unpack_mint_supply(&account.data)
}

pub fn account_exists(svm: &LiteSVM, key: &anchor_lang::prelude::Pubkey) -> bool {
    svm.get_account(key)
        .map(|account| account.lamports > 0 && !account.data.is_empty())
        .unwrap_or(false)
}

pub struct FundedDeal {
    pub svm: LiteSVM,
    pub payer: Keypair,
    pub mint_authority: Keypair,
    pub sender: Keypair,
    pub receiver: Keypair,
    pub mint: Keypair,
    pub sender_ata: anchor_lang::prelude::Pubkey,
    pub receiver_ata: anchor_lang::prelude::Pubkey,
    pub escrow: anchor_lang::prelude::Pubkey,
    pub vault: anchor_lang::prelude::Pubkey,
}

pub fn funded_deal(amount: u64) -> FundedDeal {
    let (mut svm, payer) = setup();
    let mint_authority = Keypair::new();
    let sender = Keypair::new();
    let receiver = Keypair::new();
    svm.airdrop(&sender.pubkey(), AIRDROP_LAMPORTS)
        .expect("sender airdrop");
    svm.airdrop(&receiver.pubkey(), AIRDROP_LAMPORTS)
        .expect("receiver airdrop");

    let mint = create_token(&mut svm, &payer, &mint_authority, DECIMALS);
    let sender_ata = create_token_account(&mut svm, &payer, sender.pubkey(), mint.pubkey());
    let receiver_ata = create_token_account(&mut svm, &payer, receiver.pubkey(), mint.pubkey());
    mint_tokens(
        &mut svm,
        &payer,
        &mint_authority,
        mint.pubkey(),
        sender_ata,
        amount,
    );
    initialize(
        &mut svm,
        &payer,
        &sender,
        receiver.pubkey(),
        mint.pubkey(),
        DEAL_ID,
        amount,
    );
    deposit(&mut svm, &payer, &sender, mint.pubkey(), sender_ata, DEAL_ID);

    let escrow = escrow_pda(sender.pubkey(), DEAL_ID).0;
    let vault = vault_address_for_mint(sender.pubkey(), DEAL_ID, mint.pubkey());
    FundedDeal {
        svm,
        payer,
        mint_authority,
        sender,
        receiver,
        mint,
        sender_ata,
        receiver_ata,
        escrow,
        vault,
    }
}
