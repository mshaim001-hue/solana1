mod common;

use common::{create_token, read_mint, setup, token_program, DECIMALS};
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn creates_token_2022_mint() {
    let (mut svm, payer) = setup();
    let authority = Keypair::new();

    let mint = create_token(&mut svm, &payer, &authority, DECIMALS);

    let mint_account = svm
        .get_account(&mint.pubkey())
        .expect("mint must exist");
    assert_eq!(
        mint_account.owner, token_program(),
        "mint account must be owned by Token-2022"
    );
    assert!(!mint_account.data.is_empty());

    let mint_state = read_mint(&svm, &mint.pubkey());
    assert_eq!(mint_state.decimals, DECIMALS);
    assert_eq!(mint_state.mint_authority, Some(authority.pubkey()));
    assert_eq!(mint_state.supply, 0);
}

#[test]
fn create_token_sets_requested_decimals() {
    let (mut svm, payer) = setup();
    let authority = Keypair::new();
    let custom_decimals = 9;

    let mint = create_token(&mut svm, &payer, &authority, custom_decimals);

    let mint_state = read_mint(&svm, &mint.pubkey());
    assert_eq!(mint_state.decimals, custom_decimals);
    assert_eq!(mint_state.mint_authority, Some(authority.pubkey()));
    assert_eq!(mint_state.supply, 0);
}
