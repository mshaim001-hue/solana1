mod common;

use common::{account_exists, funded_deal, mint_supply, token_amount, DEAL_AMOUNT};
use solana_signer::Signer;

#[test]
fn release_end_to_end_moves_tokens_and_closes_accounts() {
    let mut deal = funded_deal(DEAL_AMOUNT);
    let supply_before = mint_supply(&deal.svm, &deal.mint.pubkey());

    assert_eq!(token_amount(&deal.svm, &deal.vault), DEAL_AMOUNT);
    assert_eq!(token_amount(&deal.svm, &deal.sender_ata), 0);
    assert_eq!(token_amount(&deal.svm, &deal.receiver_ata), 0);
    assert!(account_exists(&deal.svm, &deal.escrow));
    assert!(account_exists(&deal.svm, &deal.vault));

    common::release(
        &mut deal.svm,
        &deal.payer,
        &deal.sender,
        deal.receiver.pubkey(),
        deal.mint.pubkey(),
        deal.receiver_ata,
        common::DEAL_ID,
    );

    assert_eq!(token_amount(&deal.svm, &deal.receiver_ata), DEAL_AMOUNT);
    assert_eq!(token_amount(&deal.svm, &deal.sender_ata), 0);
    assert_eq!(mint_supply(&deal.svm, &deal.mint.pubkey()), supply_before);
    assert!(
        !account_exists(&deal.svm, &deal.escrow),
        "escrow state must be closed after release"
    );
    assert!(
        !account_exists(&deal.svm, &deal.vault),
        "vault must be closed after release"
    );
}

#[test]
fn release_with_unsolicited_dust_drains_vault_and_closes_accounts() {
    let mut deal = funded_deal(DEAL_AMOUNT);
    common::send_dust(&mut deal);
    let supply_before = mint_supply(&deal.svm, &deal.mint.pubkey());
    assert_eq!(token_amount(&deal.svm, &deal.vault), DEAL_AMOUNT + 1);
    common::release(
        &mut deal.svm,
        &deal.payer,
        &deal.sender,
        deal.receiver.pubkey(),
        deal.mint.pubkey(),
        deal.receiver_ata,
        common::DEAL_ID,
    );
    assert_eq!(token_amount(&deal.svm, &deal.receiver_ata), DEAL_AMOUNT + 1);
    assert_eq!(token_amount(&deal.svm, &deal.sender_ata), 0);
    assert_eq!(mint_supply(&deal.svm, &deal.mint.pubkey()), supply_before);
    assert!(!account_exists(&deal.svm, &deal.vault));
    assert!(!account_exists(&deal.svm, &deal.escrow));
}
