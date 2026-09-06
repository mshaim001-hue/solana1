mod common;

use common::{
    account_exists, assert_tx_err, cancel_ix, create_token, create_token_account,
    deposit_ix, funded_deal, initialize, initialize_ix, logs_contain, mint_supply, mint_tokens,
    release, release_ix, send_instruction, setup, token_amount, vault_address_for_mint, DEAL_AMOUNT,
    DEAL_ID, DECIMALS,
};
use solana_keypair::Keypair;
use solana_signer::Signer;

fn snapshot_balances(
    svm: &litesvm::LiteSVM,
    mint: &anchor_lang::prelude::Pubkey,
    sender_ata: &anchor_lang::prelude::Pubkey,
    receiver_ata: &anchor_lang::prelude::Pubkey,
    vault: &anchor_lang::prelude::Pubkey,
) -> (u64, u64, u64, u64) {
    (
        mint_supply(svm, mint),
        token_amount(svm, sender_ata),
        token_amount(svm, receiver_ata),
        svm.get_account(vault)
            .map(|_| token_amount(svm, vault))
            .unwrap_or(0),
    )
}

#[test]
fn rejects_zero_amount() {
    let (mut svm, payer) = setup();
    let mint_authority = Keypair::new();
    let sender = Keypair::new();
    let receiver = Keypair::new();
    svm.airdrop(&sender.pubkey(), common::AIRDROP_LAMPORTS)
        .unwrap();
    svm.airdrop(&receiver.pubkey(), common::AIRDROP_LAMPORTS)
        .unwrap();
    let mint = create_token(&mut svm, &payer, &mint_authority, DECIMALS);

    let result = send_instruction(
        &mut svm,
        &payer,
        &[&payer, &sender],
        initialize_ix(&sender, receiver.pubkey(), mint.pubkey(), DEAL_ID, 0),
    );
    let error = result.expect_err("zero-amount initialize must fail");
    assert!(
        logs_contain(&error, "AmountMustBePositive")
            || logs_contain(&error, "Amount must be greater than zero"),
        "expected AmountMustBePositive, logs: {:?}",
        error.meta.logs
    );
    assert!(!account_exists(
        &svm,
        &common::escrow_pda(sender.pubkey(), DEAL_ID).0
    ));
}

#[test]
fn rejects_duplicate_deal_id() {
    let mut deal = funded_deal(DEAL_AMOUNT);
    let sender_before = token_amount(&deal.svm, &deal.sender_ata);
    let vault_before = token_amount(&deal.svm, &deal.vault);

    assert_tx_err(
        send_instruction(
            &mut deal.svm,
            &deal.payer,
            &[&deal.payer, &deal.sender],
            initialize_ix(
                &deal.sender,
                deal.receiver.pubkey(),
                deal.mint.pubkey(),
                DEAL_ID,
                DEAL_AMOUNT,
            ),
        ),
        "reusing the same sender and deal_id must fail",
    );

    assert!(account_exists(&deal.svm, &deal.escrow));
    assert_eq!(token_amount(&deal.svm, &deal.sender_ata), sender_before);
    assert_eq!(token_amount(&deal.svm, &deal.vault), vault_before);
}

#[test]
fn rejects_wrong_signer() {
    let mut deal = funded_deal(DEAL_AMOUNT);
    let before = snapshot_balances(
        &deal.svm,
        &deal.mint.pubkey(),
        &deal.sender_ata,
        &deal.receiver_ata,
        &deal.vault,
    );

    assert_tx_err(
        send_instruction(
            &mut deal.svm,
            &deal.payer,
            &[&deal.payer, &deal.receiver],
            release_ix(
                &deal.receiver,
                deal.receiver.pubkey(),
                deal.mint.pubkey(),
                deal.receiver_ata,
                DEAL_ID,
            ),
        ),
        "receiver must not be able to release",
    );

    let after = snapshot_balances(
        &deal.svm,
        &deal.mint.pubkey(),
        &deal.sender_ata,
        &deal.receiver_ata,
        &deal.vault,
    );
    assert_eq!(before, after);
    assert!(account_exists(&deal.svm, &deal.escrow));
}

#[test]
fn rejects_receiver_substitution() {
    let mut deal = funded_deal(DEAL_AMOUNT);
    let impostor = Keypair::new();
    deal.svm
        .airdrop(&impostor.pubkey(), common::AIRDROP_LAMPORTS)
        .unwrap();
    let impostor_ata =
        create_token_account(&mut deal.svm, &deal.payer, impostor.pubkey(), deal.mint.pubkey());
    let before = snapshot_balances(
        &deal.svm,
        &deal.mint.pubkey(),
        &deal.sender_ata,
        &deal.receiver_ata,
        &deal.vault,
    );

    let result = send_instruction(
        &mut deal.svm,
        &deal.payer,
        &[&deal.payer, &deal.sender],
        release_ix(
            &deal.sender,
            impostor.pubkey(),
            deal.mint.pubkey(),
            impostor_ata,
            DEAL_ID,
        ),
    );
    let error = result.expect_err("release to a substituted receiver must fail");
    assert!(
        logs_contain(&error, "ReceiverMismatch")
            || logs_contain(&error, "Receiver does not match"),
        "expected ReceiverMismatch, logs: {:?}",
        error.meta.logs
    );

    let after = snapshot_balances(
        &deal.svm,
        &deal.mint.pubkey(),
        &deal.sender_ata,
        &deal.receiver_ata,
        &deal.vault,
    );
    assert_eq!(before, after);
    assert_eq!(token_amount(&deal.svm, &impostor_ata), 0);
}

#[test]
fn rejects_mint_substitution() {
    let mut deal = funded_deal(DEAL_AMOUNT);
    let other_mint = create_token(&mut deal.svm, &deal.payer, &deal.mint_authority, DECIMALS);
    let other_sender_ata =
        create_token_account(&mut deal.svm, &deal.payer, deal.sender.pubkey(), other_mint.pubkey());
    mint_tokens(
        &mut deal.svm,
        &deal.payer,
        &deal.mint_authority,
        other_mint.pubkey(),
        other_sender_ata,
        DEAL_AMOUNT,
    );
    let before = snapshot_balances(
        &deal.svm,
        &deal.mint.pubkey(),
        &deal.sender_ata,
        &deal.receiver_ata,
        &deal.vault,
    );

    assert_tx_err(
        send_instruction(
            &mut deal.svm,
            &deal.payer,
            &[&deal.payer, &deal.sender],
            deposit_ix(&deal.sender, other_mint.pubkey(), other_sender_ata, DEAL_ID),
        ),
        "deposit with a substituted mint must fail",
    );

    let after = snapshot_balances(
        &deal.svm,
        &deal.mint.pubkey(),
        &deal.sender_ata,
        &deal.receiver_ata,
        &deal.vault,
    );
    assert_eq!(before, after);
    assert_eq!(token_amount(&deal.svm, &other_sender_ata), DEAL_AMOUNT);
}

#[test]
fn rejects_insufficient_deposit_balance() {
    let (mut svm, payer) = setup();
    let mint_authority = Keypair::new();
    let sender = Keypair::new();
    let receiver = Keypair::new();
    svm.airdrop(&sender.pubkey(), common::AIRDROP_LAMPORTS)
        .unwrap();
    svm.airdrop(&receiver.pubkey(), common::AIRDROP_LAMPORTS)
        .unwrap();
    let mint = create_token(&mut svm, &payer, &mint_authority, DECIMALS);
    let sender_ata = create_token_account(&mut svm, &payer, sender.pubkey(), mint.pubkey());
    let receiver_ata = create_token_account(&mut svm, &payer, receiver.pubkey(), mint.pubkey());
    mint_tokens(
        &mut svm,
        &payer,
        &mint_authority,
        mint.pubkey(),
        sender_ata,
        DEAL_AMOUNT / 2,
    );
    initialize(
        &mut svm,
        &payer,
        &sender,
        receiver.pubkey(),
        mint.pubkey(),
        DEAL_ID,
        DEAL_AMOUNT,
    );
    let vault = vault_address_for_mint(sender.pubkey(), DEAL_ID, mint.pubkey());

    let result = send_instruction(
        &mut svm,
        &payer,
        &[&payer, &sender],
        deposit_ix(&sender, mint.pubkey(), sender_ata, DEAL_ID),
    );
    let error = result.expect_err("deposit without a full balance must fail");
    assert!(
        logs_contain(&error, "InsufficientBalance")
            || logs_contain(&error, "Token account balance is insufficient"),
        "expected InsufficientBalance, logs: {:?}",
        error.meta.logs
    );

    assert_eq!(token_amount(&svm, &sender_ata), DEAL_AMOUNT / 2);
    assert_eq!(token_amount(&svm, &vault), 0);
    assert_eq!(token_amount(&svm, &receiver_ata), 0);
    assert!(account_exists(
        &svm,
        &common::escrow_pda(sender.pubkey(), DEAL_ID).0
    ));
}

#[test]
fn rejects_repeated_release() {
    let mut deal = funded_deal(DEAL_AMOUNT);
    release(
        &mut deal.svm,
        &deal.payer,
        &deal.sender,
        deal.receiver.pubkey(),
        deal.mint.pubkey(),
        deal.receiver_ata,
        DEAL_ID,
    );
    assert_eq!(token_amount(&deal.svm, &deal.receiver_ata), DEAL_AMOUNT);

    assert_tx_err(
        send_instruction(
            &mut deal.svm,
            &deal.payer,
            &[&deal.payer, &deal.sender],
            release_ix(
                &deal.sender,
                deal.receiver.pubkey(),
                deal.mint.pubkey(),
                deal.receiver_ata,
                DEAL_ID,
            ),
        ),
        "second release must fail because the deal is already closed",
    );

    assert_eq!(token_amount(&deal.svm, &deal.receiver_ata), DEAL_AMOUNT);
    assert!(!account_exists(&deal.svm, &deal.escrow));
    assert!(!account_exists(&deal.svm, &deal.vault));
}

#[test]
fn rejects_repeated_cancel() {
    let mut deal = funded_deal(DEAL_AMOUNT);
    common::cancel(
        &mut deal.svm,
        &deal.payer,
        &deal.sender,
        deal.mint.pubkey(),
        deal.sender_ata,
        DEAL_ID,
    );
    assert_eq!(token_amount(&deal.svm, &deal.sender_ata), DEAL_AMOUNT);

    assert_tx_err(
        send_instruction(
            &mut deal.svm,
            &deal.payer,
            &[&deal.payer, &deal.sender],
            cancel_ix(&deal.sender, deal.mint.pubkey(), deal.sender_ata, DEAL_ID),
        ),
        "second cancel must fail because the deal is already closed",
    );

    assert_eq!(token_amount(&deal.svm, &deal.sender_ata), DEAL_AMOUNT);
    assert_eq!(token_amount(&deal.svm, &deal.receiver_ata), 0);
    assert!(!account_exists(&deal.svm, &deal.escrow));
}
