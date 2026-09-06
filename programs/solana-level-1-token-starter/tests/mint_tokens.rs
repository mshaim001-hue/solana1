mod common;

use common::{
    assert_tx_err, create_token, create_token_account, logs_contain, mint_tokens, mint_tokens_ix,
    read_mint, read_token_account, send_instruction, setup, DECIMALS,
};
use solana_keypair::Keypair;
use solana_signer::Signer;

const MINT_AMOUNT: u64 = 1_000_000;

#[test]
fn mints_tokens_and_increases_supply() {
    let (mut svm, payer) = setup();
    let authority = Keypair::new();
    let owner = Keypair::new();
    let mint = create_token(&mut svm, &payer, &authority, DECIMALS);
    let destination = create_token_account(&mut svm, &payer, owner.pubkey(), mint.pubkey());

    let before_mint = read_mint(&svm, &mint.pubkey());
    let before_dest = read_token_account(&svm, &destination);
    assert_eq!(before_mint.supply, 0);
    assert_eq!(before_dest.amount, 0);

    mint_tokens(
        &mut svm,
        &payer,
        &authority,
        mint.pubkey(),
        destination,
        MINT_AMOUNT,
    );

    let after_mint = read_mint(&svm, &mint.pubkey());
    let after_dest = read_token_account(&svm, &destination);
    assert_eq!(after_mint.supply, before_mint.supply + MINT_AMOUNT);
    assert_eq!(after_dest.amount, before_dest.amount + MINT_AMOUNT);
    assert_eq!(after_dest.mint, mint.pubkey());
    assert_eq!(after_dest.owner, owner.pubkey());
}

#[test]
fn rejects_zero_mint_amount() {
    let (mut svm, payer) = setup();
    let authority = Keypair::new();
    let owner = Keypair::new();
    let mint = create_token(&mut svm, &payer, &authority, DECIMALS);
    let destination = create_token_account(&mut svm, &payer, owner.pubkey(), mint.pubkey());

    let result = send_instruction(
        &mut svm,
        &payer,
        &[&payer, &authority],
        mint_tokens_ix(&authority, mint.pubkey(), destination, 0),
    );
    let error = result.expect_err("mint_tokens with zero amount must fail");
    assert!(
        logs_contain(&error, "AmountMustBePositive")
            || logs_contain(&error, "Amount must be greater than zero"),
        "expected AmountMustBePositive, logs: {:?}",
        error.meta.logs
    );

    assert_eq!(read_mint(&svm, &mint.pubkey()).supply, 0);
    assert_eq!(read_token_account(&svm, &destination).amount, 0);
}

#[test]
fn rejects_wrong_mint_authority() {
    let (mut svm, payer) = setup();
    let authority = Keypair::new();
    let impostor = Keypair::new();
    let owner = Keypair::new();
    let mint = create_token(&mut svm, &payer, &authority, DECIMALS);
    let destination = create_token_account(&mut svm, &payer, owner.pubkey(), mint.pubkey());

    assert_tx_err(
        send_instruction(
            &mut svm,
            &payer,
            &[&payer, &impostor],
            mint_tokens_ix(&impostor, mint.pubkey(), destination, MINT_AMOUNT),
        ),
        "mint_tokens with a non-mint-authority signer must fail",
    );

    assert_eq!(read_mint(&svm, &mint.pubkey()).supply, 0);
    assert_eq!(read_token_account(&svm, &destination).amount, 0);
}

#[test]
fn rejects_destination_from_another_mint() {
    let (mut svm, payer) = setup();
    let authority = Keypair::new();
    let owner = Keypair::new();
    let mint_a = create_token(&mut svm, &payer, &authority, DECIMALS);
    let mint_b = create_token(&mut svm, &payer, &authority, DECIMALS);
    let destination_b = create_token_account(&mut svm, &payer, owner.pubkey(), mint_b.pubkey());

    assert_tx_err(
        send_instruction(
            &mut svm,
            &payer,
            &[&payer, &authority],
            mint_tokens_ix(&authority, mint_a.pubkey(), destination_b, MINT_AMOUNT),
        ),
        "mint_tokens must reject a destination token account from another mint",
    );

    assert_eq!(read_mint(&svm, &mint_a.pubkey()).supply, 0);
    assert_eq!(read_mint(&svm, &mint_b.pubkey()).supply, 0);
    assert_eq!(read_token_account(&svm, &destination_b).amount, 0);
}
