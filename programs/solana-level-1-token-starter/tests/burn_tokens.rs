mod common;

use common::{
    assert_tx_err, burn_tokens, burn_tokens_ix, create_token, create_token_account, logs_contain,
    mint_tokens, read_mint, read_token_account, send_instruction, setup, DECIMALS,
};
use solana_keypair::Keypair;
use solana_signer::Signer;

const MINT_AMOUNT: u64 = 1_000_000;
const BURN_AMOUNT: u64 = 250_000;

fn funded_burn_setup() -> (
    litesvm::LiteSVM,
    Keypair,
    Keypair,
    Keypair,
    Keypair,
    anchor_lang::prelude::Pubkey,
) {
    let (mut svm, payer) = setup();
    let mint_authority = Keypair::new();
    let owner = Keypair::new();
    let mint = create_token(&mut svm, &payer, &mint_authority, DECIMALS);
    let token_account = create_token_account(&mut svm, &payer, owner.pubkey(), mint.pubkey());
    mint_tokens(
        &mut svm,
        &payer,
        &mint_authority,
        mint.pubkey(),
        token_account,
        MINT_AMOUNT,
    );
    (svm, payer, mint_authority, owner, mint, token_account)
}

#[test]
fn burns_tokens_and_decreases_supply() {
    let (mut svm, payer, _mint_authority, owner, mint, token_account) = funded_burn_setup();

    let before_mint = read_mint(&svm, &mint.pubkey());
    let before_account = read_token_account(&svm, &token_account);
    assert_eq!(before_mint.supply, MINT_AMOUNT);
    assert_eq!(before_account.amount, MINT_AMOUNT);

    burn_tokens(
        &mut svm,
        &payer,
        &owner,
        mint.pubkey(),
        token_account,
        BURN_AMOUNT,
    );

    let after_mint = read_mint(&svm, &mint.pubkey());
    let after_account = read_token_account(&svm, &token_account);
    assert_eq!(after_mint.supply, before_mint.supply - BURN_AMOUNT);
    assert_eq!(after_account.amount, before_account.amount - BURN_AMOUNT);
    assert_eq!(after_account.mint, mint.pubkey());
    assert_eq!(after_account.owner, owner.pubkey());
}

#[test]
fn rejects_zero_burn_amount() {
    let (mut svm, payer, _mint_authority, owner, mint, token_account) = funded_burn_setup();

    let result = send_instruction(
        &mut svm,
        &payer,
        &[&payer, &owner],
        burn_tokens_ix(&owner, mint.pubkey(), token_account, 0),
    );
    let error = result.expect_err("burn_tokens with zero amount must fail");
    assert!(
        logs_contain(&error, "AmountMustBePositive")
            || logs_contain(&error, "Amount must be greater than zero"),
        "expected AmountMustBePositive, logs: {:?}",
        error.meta.logs
    );

    assert_eq!(read_mint(&svm, &mint.pubkey()).supply, MINT_AMOUNT);
    assert_eq!(read_token_account(&svm, &token_account).amount, MINT_AMOUNT);
}

#[test]
fn rejects_wrong_burn_authority() {
    let (mut svm, payer, mint_authority, _owner, mint, token_account) = funded_burn_setup();

    assert_tx_err(
        send_instruction(
            &mut svm,
            &payer,
            &[&payer, &mint_authority],
            burn_tokens_ix(&mint_authority, mint.pubkey(), token_account, BURN_AMOUNT),
        ),
        "burn_tokens with a non-token-account-owner signer must fail",
    );

    assert_eq!(read_mint(&svm, &mint.pubkey()).supply, MINT_AMOUNT);
    assert_eq!(read_token_account(&svm, &token_account).amount, MINT_AMOUNT);
}

#[test]
fn rejects_token_account_from_another_mint() {
    let (mut svm, payer, mint_authority, owner, mint, token_account) = funded_burn_setup();
    let other_mint = create_token(&mut svm, &payer, &mint_authority, DECIMALS);

    assert_tx_err(
        send_instruction(
            &mut svm,
            &payer,
            &[&payer, &owner],
            burn_tokens_ix(&owner, other_mint.pubkey(), token_account, BURN_AMOUNT),
        ),
        "burn_tokens must reject a token account from another mint",
    );

    assert_eq!(read_mint(&svm, &mint.pubkey()).supply, MINT_AMOUNT);
    assert_eq!(read_mint(&svm, &other_mint.pubkey()).supply, 0);
    assert_eq!(read_token_account(&svm, &token_account).amount, MINT_AMOUNT);
}

#[test]
fn rejects_insufficient_balance() {
    let (mut svm, payer, _mint_authority, owner, mint, token_account) = funded_burn_setup();
    let too_much = MINT_AMOUNT + 1;

    let result = send_instruction(
        &mut svm,
        &payer,
        &[&payer, &owner],
        burn_tokens_ix(&owner, mint.pubkey(), token_account, too_much),
    );
    let error = result.expect_err("burn_tokens above the account balance must fail");
    assert!(
        logs_contain(&error, "InsufficientBalance")
            || logs_contain(&error, "Token account balance is insufficient for this burn"),
        "expected InsufficientBalance, logs: {:?}",
        error.meta.logs
    );

    assert_eq!(read_mint(&svm, &mint.pubkey()).supply, MINT_AMOUNT);
    assert_eq!(read_token_account(&svm, &token_account).amount, MINT_AMOUNT);
}
