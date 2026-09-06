# Solana Level 1 Token Starter

Учебный starter Superteam KZ (уровень 1). Ветка `task/01-tests` усиливает LiteSVM-покрытие токен-программы: mint, ATA, mint/transfer и обязательные негативные сценарии.

Программа использует Token-2022 для нового токена, `anchor_spl::token_interface` для совместимости с обеими token-программами и `transfer_checked` для переводов. Тесты написаны на Rust + LiteSVM, без legacy `@solana/web3.js`.

## Зафиксированный стек

| Инструмент | Версия |
| --- | --- |
| Anchor CLI и crates | `1.1.2` |
| Solana CLI | `3.1.10` |
| Rust | `1.89.0` (`rust-toolchain.toml`) |
| LiteSVM | `0.10.0` |
| Токены | Token-2022 + `anchor_spl::token_interface` |
| Переводы | только `transfer_checked` |
| Новый TypeScript-клиент | `@solana/kit` (в этом задании клиентский код не добавлялся) |

`@solana/web3.js` не используется. TypeScript-клиент Anchor всё ещё тянет legacy SDK, поэтому тесты этого задания остаются на Rust и LiteSVM.

## Архитектура программы

Инструкции в `programs/solana-level-1-token-starter`:

- `create_token` — создаёт mint с выбранной token-программой, decimals и mint/freeze authority.
- `create_token_account` — создаёт associated token account для владельца и mint.
- `mint_tokens` — выпускает токены (`mint_to`), сумма должна быть > 0, authority должен совпадать с mint authority.
- `transfer_tokens` — переводит через `transfer_checked`; source и destination должны быть разными, mint и token program проверяются constraints.

Критичные инварианты проверяются в программе (accounts constraints + `TokenStarterError`), а не только в тестах.

## Сборка и тесты

Чистый checkout этой ветки должен проходить:

```bash
anchor build --ignore-keys
cargo test --workspace --locked
```

Флаг `--ignore-keys` нужен потому, что program keypair намеренно не хранится в репозитории. Для собственного devnet-деплоя создайте keypair локально и выполните `anchor keys sync`, но не коммитьте файл ключа.

Тесты читают `target/deploy/solana_level_1_token_starter.so`, поэтому `anchor build --ignore-keys` обязателен перед первым `cargo test`.

### Ожидаемый результат

- `anchor build --ignore-keys` завершается успешно и пишет `.so` в `target/deploy/`.
- `cargo test --workspace --locked` проходит все тесты:
  - `creates_token_2022_mint`, `create_token_sets_requested_decimals`
  - `creates_associated_token_account`
  - `mints_tokens_and_increases_supply` и негативные mint-сценарии
  - `transfers_tokens_without_changing_supply` и негативные transfer-сценарии

## Что покрывают тесты

Положительные сценарии:

- `create_token` — владелец mint (Token-2022), `decimals`, mint authority и нулевой `supply`.
- `create_token_account` — ATA принадлежит Token-2022, поля `owner` и `mint` совпадают с аргументами, баланс 0.
- `mint_tokens` — баланс получателя и общий `supply` увеличиваются на одну и ту же сумму.
- `transfer_tokens` — баланс source уменьшается, destination увеличивается, `supply` не меняется.

Негативные сценарии:

- нулевая сумма (`mint_tokens`, `transfer_tokens`) → `AmountMustBePositive`
- неверный authority (не mint authority / не владелец source)
- другой mint (destination ATA от другого mint)
- одинаковые source и destination → `SourceEqualsDestination`

Общие хелперы лежат в `programs/solana-level-1-token-starter/tests/common/mod.rs`.

## Правила сдачи

- публичная ссылка на GitHub и ветка `task/01-tests` (или commit SHA);
- не публикуйте keypair, seed phrase, приватные ключи или `.env` с секретами;
- не используйте `@solana/web3.js` в новом клиентском коде;
- для переводов используйте `transfer_checked`.

Следующие задания выполняются в ветках `task/02-burn` и `task/03-escrow`.
