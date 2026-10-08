//! The RPC node resolves (SUI) or assembles (IOTA) every transaction, so its bytes are
//! checked before signing: the sender pays, only expected packages and shared objects are
//! touched, and funds can only go to the sender or to recipients the user chose.

use anyhow::{Result, anyhow, bail, ensure};
use sui_sdk_types::{Address, Command, Input, Transaction, TransactionKind, WithdrawFrom};

pub struct Expected {
    pub sender: Address,
    pub recipients: Vec<Address>,
    /// Packages besides the framework (0x1, 0x2, 0x3) the transaction may call.
    pub packages: Vec<Address>,
    pub shared_objects: Vec<Address>,
    /// Object IDs passed by value, such as a Walrus node ID.
    pub ids: Vec<Address>,
    /// The most a misbehaving node could make the sender burn on gas.
    pub max_budget: u64,
}

impl Expected {
    pub fn new(sender: Address, max_budget: u64) -> Self {
        Self {
            sender,
            recipients: Vec::new(),
            packages: Vec::new(),
            shared_objects: Vec::new(),
            ids: Vec::new(),
            max_budget,
        }
    }
}

const FRAMEWORK: [Address; 3] = [Address::from_static("0x1"), Address::TWO, Address::THREE];

pub fn check(tx: &Transaction, expected: &Expected) -> Result<()> {
    ensure!(
        tx.sender == expected.sender,
        "the node built a transaction for {} instead of {}",
        tx.sender,
        expected.sender
    );
    ensure!(
        tx.gas_payment.owner == expected.sender,
        "the node made {} pay for gas",
        tx.gas_payment.owner
    );
    ensure!(
        tx.gas_payment.budget <= expected.max_budget,
        "the node set a gas budget of {} base units, above the limit of {}; pass --gas-budget to set it yourself",
        tx.gas_payment.budget,
        expected.max_budget
    );
    let TransactionKind::ProgrammableTransaction(ptb) = &tx.kind else {
        bail!("the node returned an unexpected kind of transaction");
    };
    for command in &ptb.commands {
        match command {
            Command::MoveCall(call) => ensure!(
                FRAMEWORK.contains(&call.package) || expected.packages.contains(&call.package),
                "unexpected call to {}::{}::{}",
                call.package,
                call.module,
                call.function
            ),
            Command::TransferObjects(_)
            | Command::SplitCoins(_)
            | Command::MergeCoins(_)
            | Command::MakeMoveVector(_) => {}
            other => bail!("unexpected command {other:?}"),
        }
    }
    for input in &ptb.inputs {
        match input {
            // Our transactions pass addresses and IDs (32 bytes), amounts (u64) and Option<u64>.
            Input::Pure(bytes) if bytes.len() == 32 => {
                let address = Address::from_bytes(bytes).map_err(|_| anyhow!("bad address"))?;
                ensure!(
                    address == expected.sender
                        || expected.recipients.contains(&address)
                        || expected.ids.contains(&address),
                    "the transaction would send funds to {address}, which you did not choose"
                );
            }
            Input::Pure(bytes) if matches!(bytes.len(), 1 | 8 | 9) => {}
            Input::Pure(bytes) => bail!("unexpected {}-byte argument", bytes.len()),
            Input::ImmutableOrOwned(_) | Input::Receiving(_) => {}
            Input::Shared(shared) => ensure!(
                expected.shared_objects.contains(&shared.object_id()),
                "unexpected shared object {}",
                shared.object_id()
            ),
            Input::FundsWithdrawal(withdrawal) => ensure!(
                matches!(withdrawal.source(), WithdrawFrom::Sender),
                "unexpected source of funds"
            ),
            other => bail!("unexpected input {other:?}"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use sui_sdk_types::Digest;
    use sui_transaction_builder::{ObjectInput, TransactionBuilder};

    const SENDER: Address = Address::from_static("0xa1");
    const FRIEND: Address = Address::from_static("0xb2");
    const STRANGER: Address = Address::from_static("0xc3");

    fn transfer_to(recipient: Address, budget: u64) -> Transaction {
        let mut tx = TransactionBuilder::new();
        let amount = tx.pure(&1_000u64);
        let gas = tx.gas();
        let coins = tx.split_coins(gas, vec![amount]);
        let recipient = tx.pure(&recipient);
        tx.transfer_objects(coins, recipient);
        tx.set_sender(SENDER);
        tx.set_gas_budget(budget);
        tx.set_gas_price(1000);
        tx.add_gas_objects([ObjectInput::owned(
            Address::from_static("0x99"),
            1,
            Digest::ZERO,
        )]);
        tx.try_build().unwrap()
    }

    fn expected() -> Expected {
        let mut expected = Expected::new(SENDER, 50_000_000);
        expected.recipients.push(FRIEND);
        expected
    }

    #[test]
    fn allows_the_chosen_recipient() {
        check(&transfer_to(FRIEND, 10_000_000), &expected()).unwrap();
        check(&transfer_to(SENDER, 10_000_000), &expected()).unwrap();
    }

    #[test]
    fn rejects_other_recipients_and_large_budgets() {
        assert!(check(&transfer_to(STRANGER, 10_000_000), &expected()).is_err());
        assert!(check(&transfer_to(FRIEND, 60_000_000), &expected()).is_err());
        let mut other_sender = expected();
        other_sender.sender = FRIEND;
        assert!(check(&transfer_to(FRIEND, 10_000_000), &other_sender).is_err());
    }

    #[test]
    fn allows_listed_ids() {
        let node = Address::from_static("0xd4");
        let mut expected = expected();
        assert!(check(&transfer_to(node, 10_000_000), &expected).is_err());
        expected.ids.push(node);
        check(&transfer_to(node, 10_000_000), &expected).unwrap();
    }

    #[test]
    fn checks_iota_transactions_built_by_the_node() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(include_str!("../tests/data/iota_tx.b64").trim())
            .unwrap();
        let tx: Transaction = bcs::from_bytes(&bytes).unwrap();
        let sender = "0x7d307e5537bf0ebd7417f0aa2b09562e5be2a3b1bc0df39ecba8df606c6002b9";
        let mut expected = Expected::new(sender.parse().unwrap(), 50_000_000);
        assert!(check(&tx, &expected).is_err(), "0x5 is not allowed yet");
        expected.shared_objects.push(Address::from_static("0x5"));
        check(&tx, &expected).unwrap();
    }
}
