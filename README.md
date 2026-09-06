# Solana Level 1 Token Starter

Учебный starter Superteam KZ (уровень 1). Ветка `task/02-burn` добавляет инструкцию `burn_tokens` поверх покрытия из `task/01-tests`.

Программа использует Token-2022 для нового токена, `anchor_spl::token_interface` для совместимости с обеими token-программами, `transfer_checked` для переводов и `burn_checked` для сжигания. Тесты написаны на Rust + LiteSVM, без legacy `@solana/web3.js`.

## Зафиксированный стек

| Инструмент | Версия |
| --- | --- |
| Anchor CLI и crates | `1.1.2` |
| Solana CLI | `3.1.10` |
| Rust | `1.89.0` (`rust-toolchain.toml`) |
| LiteSVM | `0.10.0` |
| Токены | Token-2022 + `anchor_spl::token_interface` |
| Переводы | только `transfer_checked` |
| Сжигание | только `burn_checked` с `decimals` из mint |
| Новый TypeScript-клиент | `@solana/kit` (клиентский код не добавлялся) |

## Архитектура программы

- `create_token` — создаёт mint с выбранной token-программой, decimals и mint/freeze authority.
- `create_token_account` — создаёт associated token account для владельца и mint.
- `mint_tokens` — выпускает токены (`mint_to`), сумма должна быть > 0.
- `transfer_tokens` — переводит через `transfer_checked`.
- `burn_tokens` — сжигает токены через `burn_checked`.

Критичные инварианты проверяются в программе (accounts constraints + `TokenStarterError`), а не только в тестах. Критичные аккаунты типизированы (`Signer`, `InterfaceAccount`, `Interface`), без `UncheckedAccount`.

## Account constraints `burn_tokens`

| Аккаунт | Проверки |
| --- | --- |
| `authority` | `Signer` — владелец token account должен подписать транзакцию |
| `mint` | `mut`, `InterfaceAccount<Mint>`, `mint::token_program = token_program` |
| `token_account` | `mut`, `InterfaceAccount<TokenAccount>`, `token::mint = mint`, `token::authority = authority`, `token::token_program = token_program` |
| `token_program` | `Interface<TokenInterface>` — Token-2022 или классический Token Program |

В handler дополнительно:

- `amount > 0` → `TokenStarterError::AmountMustBePositive`
- `token_account.amount >= amount` → `TokenStarterError::InsufficientBalance`
- CPI `token_interface::burn_checked(..., amount, mint.decimals)`

Неверный authority, чужой mint или другая token program отклоняются constraints до CPI. После любой ошибки баланс token account и `supply` mint не меняются.

## Сборка и тесты

Чистый checkout этой ветки должен проходить:

```bash
anchor build
cargo test --workspace --locked
```

Если `anchor build` предупредит о несовпадении program ID (локальный keypair генерируется в `target/deploy/` и не коммитится), повторите сборку так:

```bash
anchor build --ignore-keys
```

Тесты читают `target/deploy/solana_level_1_token_starter.so`, поэтому перед первым `cargo test` нужна сборка.

### Ожидаемый результат

- `anchor build` пишет `.so` в `target/deploy/`.
- `cargo test --workspace --locked` проходит тесты первого задания и новые burn-тесты:
  - `burns_tokens_and_decreases_supply`
  - `rejects_zero_burn_amount`
  - `rejects_wrong_burn_authority`
  - `rejects_token_account_from_another_mint`
  - `rejects_insufficient_balance`

## Что покрывают тесты `burn_tokens`

- успешное сжигание уменьшает баланс token account и общий `supply` на одну и ту же сумму;
- нулевая сумма → `AmountMustBePositive`, состояние не меняется;
- неверный authority (не владелец token account) отклоняется, состояние не меняется;
- token account от другого mint отклоняется, состояние не меняется;
- сумма больше баланса → `InsufficientBalance`, состояние не меняется.

## Правила сдачи

- публичная ссылка на GitHub и ветка `task/02-burn` (или commit SHA);
- не публикуйте keypair, seed phrase, приватные ключи или `.env` с секретами;
- не используйте `@solana/web3.js` в новом клиентском коде;
- для переводов используйте `transfer_checked`, для burn — `burn_checked`.
