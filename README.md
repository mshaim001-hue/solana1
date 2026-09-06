# Solana Level 1 — Token starter + Escrow

Учебный workspace Superteam KZ. Ветка `task/03-escrow` добавляет программу `programs/escrow`: минимальное DeFi-приложение, которое блокирует Token-2022 токены отправителя до `release` или `cancel`.

## Зафиксированный стек

| Инструмент | Версия |
| --- | --- |
| Anchor CLI и crates | `1.1.2` |
| Solana CLI | `3.1.10` |
| Rust | `1.89.0` (`rust-toolchain.toml`) |
| LiteSVM | `0.10.0` |
| Токены | Token-2022 + `anchor_spl::token_interface` |
| Переводы / burn / escrow CPI | только `transfer_checked` / `burn_checked` |
| Новый TypeScript | `@solana/kit` (не добавлялся) |

## Архитектура escrow

Каждая сделка — отдельный PDA и отдельный vault.

- **EscrowState** (`seeds = [b"escrow", sender, deal_id.to_le_bytes()]`) хранит `sender`, `receiver`, `mint`, `amount`, `deal_id`, `bump`, `status`.
- **Vault** — ATA этого PDA: authority = escrow PDA, mint и token program зафиксированы constraints. Общего vault нет.
- Критичные аккаунты типизированы (`Signer`, `Account`, `InterfaceAccount`, `SystemAccount`). `UncheckedAccount` не используется.

### State machine

```
initialize(amount > 0, receiver != sender)
        │
        ▼
     Created ──cancel──► Cancelled (vault пустой, оба аккаунта закрываются)
        │
     deposit (точная сумма через transfer_checked)
        │
        ▼
      Funded ──release──► Released (токены в ATA receiver, vault+state закрываются)
        │
     cancel
        │
        ▼
     Cancelled (токены возвращаются sender, vault+state закрываются)
```

Каждый переход выполняется один раз. `Released` и `Cancelled` терминальны: аккаунты закрываются, rent уходит `sender`. Повторный `release`/`cancel` невозможен.

### Инструкции

| Инструкция | Кто | Что проверяет программа |
| --- | --- | --- |
| `initialize(deal_id, amount)` | sender | `amount > 0`, `receiver != sender`, уникальный PDA, vault привязан к mint/token program |
| `deposit(deal_id)` | sender | status `Created`, `has_one = sender/mint`, seeds/bump, sender ATA, vault authority = PDA, баланс ≥ amount, CPI `transfer_checked` ровно `amount` |
| `release(deal_id)` | только sender | status `Funded`, receiver совпадает со state, mint/token accounts, PDA-подписанный `transfer_checked` в ATA receiver, close vault + state |
| `cancel(deal_id)` | только sender | status `Created` или `Funded`, возврат токенов если были, close vault + state |

### Threat model

| Угроза | Защита |
| --- | --- |
| Украсть токены из чужой сделки | vault authority = PDA сделки; CPI только с seeds этой сделки |
| Общий vault / смешение сделок | уникальный PDA `[escrow, sender, deal_id]`, vault — ATA этого PDA |
| Повторный release/cancel | одноразовый статус + закрытие аккаунтов |
| Подмена mint | `has_one = mint`, `token::mint`, `mint::token_program` |
| Подмена receiver | `receiver.key() == escrow.receiver` и ATA authority = receiver |
| Неверный signer | `Signer` + `has_one = sender`; receiver не может deposit/release/cancel |
| Нулевая сумма | `AmountMustBePositive` в initialize |
| Недостаточный баланс | `InsufficientBalance` до CPI |
| Повторный deal_id | `init` того же PDA отклоняется |
| Unchecked transfer | запрещён; только `transfer_checked` |

После любой отклонённой транзакции балансы, supply и escrow-аккаунты не меняются.

## Сборка и тесты

```bash
anchor build --ignore-keys
cargo test --workspace --locked
```

`anchor build` тоже проходит; `--ignore-keys` нужен, потому что program keypair не хранится в Git.

### Ожидаемый результат

- собираются `solana_level_1_token_starter` и `escrow`;
- проходят тесты токен-программы (task 01–02) и escrow:
  - `release_end_to_end_moves_tokens_and_closes_accounts`
  - `cancel_end_to_end_returns_tokens_and_closes_accounts`
  - негативы: нулевая сумма, повторный deal_id, неверный signer, подмена receiver/mint, недостаточный баланс, повторные release/cancel.

## Правила сдачи

- публичная ссылка и ветка `task/03-escrow` (или commit SHA);
- не публикуйте keypair, seed phrase, приватные ключи или `.env`.
