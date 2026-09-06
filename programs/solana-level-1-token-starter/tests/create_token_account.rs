mod common;

use common::{
    associated_token_address, create_token, create_token_account, read_token_account, setup,
    token_program, DECIMALS,
};
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn creates_associated_token_account() {
    let (mut svm, payer) = setup();
    let authority = Keypair::new();
    let owner = Keypair::new();
    let mint = create_token(&mut svm, &payer, &authority, DECIMALS);

    let token_account = create_token_account(&mut svm, &payer, owner.pubkey(), mint.pubkey());

    assert_eq!(
        token_account,
        associated_token_address(owner.pubkey(), mint.pubkey())
    );

    let account = svm
        .get_account(&token_account)
        .expect("token account must exist");
    assert_eq!(
        account.owner, token_program(),
        "token account must be owned by Token-2022"
    );

    let state = read_token_account(&svm, &token_account);
    assert_eq!(state.owner, owner.pubkey());
    assert_eq!(state.mint, mint.pubkey());
    assert_eq!(state.amount, 0);
}
