mod common;

use common::{
    account_exists, funded_deal, mint_supply, token_amount, DEAL_AMOUNT,
};
use solana_signer::Signer;

#[test]
fn cancel_end_to_end_returns_tokens_and_closes_accounts() {
    let mut deal = funded_deal(DEAL_AMOUNT);
    let supply_before = mint_supply(&deal.svm, &deal.mint.pubkey());

    assert_eq!(token_amount(&deal.svm, &deal.vault), DEAL_AMOUNT);
    assert_eq!(token_amount(&deal.svm, &deal.sender_ata), 0);

    common::cancel(
        &mut deal.svm,
        &deal.payer,
        &deal.sender,
        deal.mint.pubkey(),
        deal.sender_ata,
        common::DEAL_ID,
    );

    assert_eq!(token_amount(&deal.svm, &deal.sender_ata), DEAL_AMOUNT);
    assert_eq!(token_amount(&deal.svm, &deal.receiver_ata), 0);
    assert_eq!(mint_supply(&deal.svm, &deal.mint.pubkey()), supply_before);
    assert!(
        !account_exists(&deal.svm, &deal.escrow),
        "escrow state must be closed after cancel"
    );
    assert!(
        !account_exists(&deal.svm, &deal.vault),
        "vault must be closed after cancel"
    );
}
