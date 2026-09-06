mod common;

use common::{
    assert_tx_err, create_token, create_token_account, logs_contain, mint_tokens, read_mint,
    read_token_account, send_instruction, setup, transfer_tokens, transfer_tokens_ix, DECIMALS,
};
use solana_keypair::Keypair;
use solana_signer::Signer;

const MINT_AMOUNT: u64 = 1_000_000;
const TRANSFER_AMOUNT: u64 = 250_000;

fn funded_transfer_setup() -> (
    litesvm::LiteSVM,
    Keypair,
    Keypair,
    Keypair,
    Keypair,
    Keypair,
    anchor_lang::prelude::Pubkey,
    anchor_lang::prelude::Pubkey,
) {
    let (mut svm, payer) = setup();
    let mint_authority = Keypair::new();
    let source_owner = Keypair::new();
    let destination_owner = Keypair::new();
    let mint = create_token(&mut svm, &payer, &mint_authority, DECIMALS);
    let source = create_token_account(&mut svm, &payer, source_owner.pubkey(), mint.pubkey());
    let destination =
        create_token_account(&mut svm, &payer, destination_owner.pubkey(), mint.pubkey());
    mint_tokens(
        &mut svm,
        &payer,
        &mint_authority,
        mint.pubkey(),
        source,
        MINT_AMOUNT,
    );
    (
        svm,
        payer,
        mint_authority,
        source_owner,
        destination_owner,
        mint,
        source,
        destination,
    )
}

#[test]
fn transfers_tokens_without_changing_supply() {
    let (mut svm, payer, _mint_authority, source_owner, destination_owner, mint, source, destination) =
        funded_transfer_setup();

    let before_supply = read_mint(&svm, &mint.pubkey()).supply;
    let before_source = read_token_account(&svm, &source).amount;
    let before_destination = read_token_account(&svm, &destination).amount;

    transfer_tokens(
        &mut svm,
        &payer,
        &source_owner,
        mint.pubkey(),
        source,
        destination,
        TRANSFER_AMOUNT,
    );

    let after_mint = read_mint(&svm, &mint.pubkey());
    let after_source = read_token_account(&svm, &source);
    let after_destination = read_token_account(&svm, &destination);

    assert_eq!(after_mint.supply, before_supply);
    assert_eq!(after_source.amount, before_source - TRANSFER_AMOUNT);
    assert_eq!(
        after_destination.amount,
        before_destination + TRANSFER_AMOUNT
    );
    assert_eq!(after_source.mint, mint.pubkey());
    assert_eq!(after_destination.mint, mint.pubkey());
    assert_eq!(after_source.owner, source_owner.pubkey());
    assert_eq!(after_destination.owner, destination_owner.pubkey());
}

#[test]
fn rejects_zero_transfer_amount() {
    let (mut svm, payer, _mint_authority, source_owner, _destination_owner, mint, source, destination) =
        funded_transfer_setup();

    let result = send_instruction(
        &mut svm,
        &payer,
        &[&payer, &source_owner],
        transfer_tokens_ix(&source_owner, mint.pubkey(), source, destination, 0),
    );
    let error = result.expect_err("transfer_tokens with zero amount must fail");
    assert!(
        logs_contain(&error, "AmountMustBePositive")
            || logs_contain(&error, "Amount must be greater than zero"),
        "expected AmountMustBePositive, logs: {:?}",
        error.meta.logs
    );

    assert_eq!(read_token_account(&svm, &source).amount, MINT_AMOUNT);
    assert_eq!(read_token_account(&svm, &destination).amount, 0);
    assert_eq!(read_mint(&svm, &mint.pubkey()).supply, MINT_AMOUNT);
}

#[test]
fn rejects_wrong_source_authority() {
    let (mut svm, payer, _mint_authority, _source_owner, destination_owner, mint, source, destination) =
        funded_transfer_setup();

    assert_tx_err(
        send_instruction(
            &mut svm,
            &payer,
            &[&payer, &destination_owner],
            transfer_tokens_ix(
                &destination_owner,
                mint.pubkey(),
                source,
                destination,
                TRANSFER_AMOUNT,
            ),
        ),
        "transfer_tokens with a non-source-owner signer must fail",
    );

    assert_eq!(read_token_account(&svm, &source).amount, MINT_AMOUNT);
    assert_eq!(read_token_account(&svm, &destination).amount, 0);
}

#[test]
fn rejects_destination_from_another_mint() {
    let (mut svm, payer, mint_authority, source_owner, destination_owner, mint, source, _destination) =
        funded_transfer_setup();
    let other_mint = create_token(&mut svm, &payer, &mint_authority, DECIMALS);
    let other_destination =
        create_token_account(&mut svm, &payer, destination_owner.pubkey(), other_mint.pubkey());

    assert_tx_err(
        send_instruction(
            &mut svm,
            &payer,
            &[&payer, &source_owner],
            transfer_tokens_ix(
                &source_owner,
                mint.pubkey(),
                source,
                other_destination,
                TRANSFER_AMOUNT,
            ),
        ),
        "transfer_tokens must reject a destination token account from another mint",
    );

    assert_eq!(read_token_account(&svm, &source).amount, MINT_AMOUNT);
    assert_eq!(read_token_account(&svm, &other_destination).amount, 0);
    assert_eq!(read_mint(&svm, &mint.pubkey()).supply, MINT_AMOUNT);
}

#[test]
fn rejects_same_source_and_destination() {
    let (mut svm, payer, _mint_authority, source_owner, _destination_owner, mint, source, _destination) =
        funded_transfer_setup();

    let result = send_instruction(
        &mut svm,
        &payer,
        &[&payer, &source_owner],
        transfer_tokens_ix(
            &source_owner,
            mint.pubkey(),
            source,
            source,
            TRANSFER_AMOUNT,
        ),
    );
    let error = result.expect_err("transfer_tokens to the same account must fail");
    assert!(
        logs_contain(&error, "SourceEqualsDestination")
            || logs_contain(&error, "Source and destination token accounts must be different")
            || logs_contain(&error, "ConstraintDuplicateMutableAccount"),
        "expected same-account transfer to be rejected, logs: {:?}",
        error.meta.logs
    );

    assert_eq!(read_token_account(&svm, &source).amount, MINT_AMOUNT);
    assert_eq!(read_mint(&svm, &mint.pubkey()).supply, MINT_AMOUNT);
}
