use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use ex07_structs_and_traits::{
    Account, AccountError, BoxError, CartError, CheckingAccount, Currency, Email, EmailNotifier,
    Employee, EmployeeError, EmployeeRecord, Ledger, Money, MoneyError, NotificationDispatcher,
    NotificationError, Notifier, Part, PreIdentifier, Product, ProductError, SMS_MAX_CHARS,
    SavingsAccount, ShoppingCart, Sku, SmsNotifier, Version, VersionError, WebhookNotifier,
};
use proptest::prelude::*;

fn currency(code: &str) -> Currency {
    Currency::try_from(code).expect("valid test currency")
}

fn usd(minor: i64) -> Money {
    Money::new(minor, currency("USD"))
}

fn eur(minor: i64) -> Money {
    Money::new(minor, currency("EUR"))
}

fn sku(raw: &str) -> Sku {
    Sku::new(raw).expect("valid test SKU")
}

fn product(code: &str, price: Money) -> Product {
    Product::new(sku(code), "Widget", price).expect("valid test product")
}

fn cart_usd() -> ShoppingCart {
    ShoppingCart::new(currency("USD"))
}

fn v(text: &str) -> Version {
    Version::parse(text).expect("valid test version")
}

fn hash_of(version: &Version) -> u64 {
    let mut hasher = DefaultHasher::new();
    version.hash(&mut hasher);
    hasher.finish()
}

fn email_channel() -> Box<dyn Notifier> {
    Box::new(EmailNotifier::new(|_: &str, _: &str| -> Result<(), BoxError> {
        Ok(())
    }))
}

// ---------------------------------------------------------------------------
// Part 1: Money
// ---------------------------------------------------------------------------

#[test]
fn part1_currency_accepts_three_uppercase_letters() {
    let usd = Currency::try_from("USD").expect("USD is a valid code");
    assert_eq!(usd.as_str(), "USD", "as_str returns the code");
    assert_eq!(
        "EUR".parse::<Currency>().map(|c| c.to_string()),
        Ok("EUR".to_owned()),
        "FromStr applies the same rule"
    );
}

#[test]
fn part1_currency_rejects_bad_codes() {
    for bad in ["usd", "US", "USDX", "U5D", "", " USD"] {
        assert!(
            matches!(Currency::try_from(bad), Err(MoneyError::InvalidCurrency(_))),
            "{bad:?} should be rejected as InvalidCurrency"
        );
    }
}

#[test]
fn part1_currency_minor_digits_and_display() {
    assert_eq!(currency("JPY").minor_digits(), 0, "JPY has no minor unit");
    assert_eq!(currency("KRW").minor_digits(), 0, "KRW has no minor unit");
    assert_eq!(currency("KWD").minor_digits(), 3, "KWD uses three digits");
    assert_eq!(currency("USD").minor_digits(), 2, "USD uses two digits");
    assert_eq!(currency("USD").to_string(), "USD", "Display is the bare code");
    assert_eq!(format!("{:?}", currency("USD")), "Currency(\"USD\")", "Debug names the type");
}

#[test]
fn part1_money_parses_amounts_with_trailing_zeros() {
    assert_eq!("10.50 USD".parse::<Money>(), Ok(usd(1050)), "10.50 USD is 1050 minor units");
    assert_eq!("-0.05 USD".parse::<Money>(), Ok(usd(-5)), "a leading minus keeps its sign");
    assert_eq!(
        "100 JPY".parse::<Money>(),
        Ok(Money::new(100, currency("JPY"))),
        "JPY has no fraction part"
    );
    assert_eq!("1.500 USD".parse::<Money>(), Ok(usd(150)), "zeros beyond the minor unit are fine");
}

#[test]
fn part1_money_rejects_excess_precision() {
    assert!(
        matches!("1.005 USD".parse::<Money>(), Err(MoneyError::TooPrecise(_))),
        "a non-zero third digit is too precise for USD"
    );
    assert!(
        matches!("100.5 JPY".parse::<Money>(), Err(MoneyError::TooPrecise(_))),
        "JPY allows no fraction digits"
    );
}

#[test]
fn part1_money_rejects_malformed_text() {
    for bad in ["1,000.00 USD", "10. USD", ".5 USD", "1e3 USD", "10.00"] {
        assert!(
            matches!(bad.parse::<Money>(), Err(MoneyError::InvalidAmount(_))),
            "{bad:?} should be rejected as InvalidAmount"
        );
    }
    assert!(
        matches!("10.00 usd".parse::<Money>(), Err(MoneyError::InvalidCurrency(_))),
        "a bad currency code is InvalidCurrency"
    );
}

#[test]
fn part1_money_rejects_overflowing_whole_part() {
    assert_eq!(
        "9999999999999999999 USD".parse::<Money>(),
        Err(MoneyError::Overflow),
        "a whole part that does not fit i64 overflows"
    );
}

#[test]
fn part1_money_display_keeps_sign_and_minor_digits() {
    assert_eq!(usd(1050).to_string(), "10.50 USD");
    assert_eq!(usd(-5).to_string(), "-0.05 USD", "the sign survives a zero major part");
    assert_eq!(Money::new(100, currency("JPY")).to_string(), "100 JPY");
    assert_eq!(usd(7).to_string(), "0.07 USD");
}

#[test]
fn part1_checked_ops_reject_mismatch_and_overflow() {
    assert_eq!(
        usd(1).checked_add(eur(1)),
        Err(MoneyError::CurrencyMismatch {
            left: currency("USD"),
            right: currency("EUR"),
        }),
        "addition across currencies is an error; left is self"
    );
    assert!(matches!(usd(1).checked_sub(eur(1)), Err(MoneyError::CurrencyMismatch { .. })));
    assert!(matches!(usd(1).checked_cmp(eur(1)), Err(MoneyError::CurrencyMismatch { .. })));
    assert_eq!(usd(i64::MAX).checked_add(usd(1)), Err(MoneyError::Overflow));
    assert_eq!(usd(i64::MIN).checked_sub(usd(1)), Err(MoneyError::Overflow));
    assert_eq!(usd(7).checked_cmp(usd(9)), Ok(Ordering::Less));
}

#[test]
#[should_panic(expected = "Money addition failed")]
fn part1_add_operator_panics_on_currency_mismatch() {
    let _ = usd(1) + eur(1);
}

#[test]
fn part1_partial_ord_is_none_across_currencies() {
    assert_eq!(usd(1).partial_cmp(&eur(1)), None, "different currencies are unordered");
    assert!(usd(1) < usd(2), "the same currency compares by amount");
    assert_eq!(usd(3).partial_cmp(&usd(3)), Some(Ordering::Equal));
}

#[test]
fn part1_scaled_rounds_half_to_even() {
    // (minor, numerator, denominator, expected minor)
    let cases: [(i64, i64, i64, i64); 7] = [
        (5, 1, 2, 2),      // 2.5 -> 2
        (7, 1, 2, 4),      // 3.5 -> 4
        (1, 1, 2, 0),      // 0.5 -> 0
        (-5, 1, 2, -2),    // -2.5 -> -2
        (25, 1, 10, 2),    // 2.5 -> 2
        (15, 1, 10, 2),    // 1.5 -> 2
        (1000, 1, 3, 333), // 333.33 -> 333
    ];
    for (minor, numerator, denominator, expected) in cases {
        assert_eq!(
            usd(minor).scaled(numerator, denominator),
            Ok(usd(expected)),
            "{minor} * {numerator}/{denominator} should round half to even"
        );
    }
    assert_eq!(usd(5).scaled(1, 0), Err(MoneyError::ZeroDenominator));
}

#[test]
fn part1_allocate_gives_remainder_to_first_shares() {
    assert_eq!(
        usd(1000).allocate(3),
        Ok(vec![usd(334), usd(333), usd(333)]),
        "the first shares absorb the remainder"
    );
    assert_eq!(
        usd(-10).allocate(3),
        Ok(vec![usd(-4), usd(-3), usd(-3)]),
        "negative amounts split by magnitude and keep the sign"
    );
    assert_eq!(usd(5).allocate(0), Err(MoneyError::ZeroParts));
}

#[test]
fn part1_try_sum_checks_currency_and_empty_is_zero() {
    assert_eq!(Money::try_sum(currency("USD"), [usd(150), usd(250)]), Ok(usd(400)));
    assert_eq!(
        Money::try_sum(currency("USD"), Vec::<Money>::new()),
        Ok(Money::zero(currency("USD"))),
        "an empty sum is zero in the given currency"
    );
    assert!(
        matches!(
            Money::try_sum(currency("USD"), [usd(1), eur(1)]),
            Err(MoneyError::CurrencyMismatch { .. })
        ),
        "a foreign item is a currency mismatch"
    );
}

#[test]
fn part1_mul_and_neg_operators() {
    assert_eq!(usd(250) * 3, usd(750));
    assert_eq!(4 * usd(250), usd(1000));
    assert_eq!(-usd(5), usd(-5));
}

#[test]
#[should_panic(expected = "Money multiplication overflowed")]
fn part1_mul_operator_panics_on_overflow() {
    let _ = usd(i64::MAX) * 2;
}

#[test]
#[should_panic(expected = "Money negation overflowed")]
fn part1_neg_operator_panics_on_i64_min() {
    let _ = -usd(i64::MIN);
}

proptest! {
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn part1_allocate_sums_exactly_and_evenly(
        minor in -1_000_000_000i64..1_000_000_000,
        parts in 1usize..64,
    ) {
        let shares = usd(minor).allocate(parts).expect("parts is non-zero");
        prop_assert_eq!(shares.len(), parts);
        let total: i64 = shares.iter().map(|share| share.minor()).sum();
        prop_assert_eq!(total, minor, "shares must sum to the original amount");
        let magnitudes: Vec<i64> = shares.iter().map(|share| share.minor().abs()).collect();
        let hi = magnitudes.iter().copied().max().unwrap_or(0);
        let lo = magnitudes.iter().copied().min().unwrap_or(0);
        prop_assert!(hi - lo <= 1, "shares differ by at most one minor unit");
    }
}

// ---------------------------------------------------------------------------
// Part 2: Sku and Product
// ---------------------------------------------------------------------------

#[test]
fn part2_sku_accepts_and_rejects_by_shape() {
    for ok in ["ABCD", "A-1-B-2", "ABCDEFGHIJKLMNOP"] {
        assert!(Sku::new(ok).is_ok(), "{ok:?} should be a valid SKU");
    }
    for bad in ["ABC", "abcd", "AB CD", "ABCDEFGHIJKLMNOPQ", "AB_CD", " ABCD", "ÄBCD"] {
        assert!(
            matches!(Sku::new(bad), Err(ProductError::InvalidSku(_))),
            "{bad:?} should be rejected"
        );
    }
    assert_eq!("ABCD".parse::<Sku>().expect("valid").as_str(), "ABCD", "FromStr uses the same rule");
}

#[test]
fn part2_product_trims_name_and_starts_out_of_stock() {
    let lamp = Product::new(sku("LAMP-01"), "  Desk Lamp  ", usd(1999)).expect("valid product");
    assert_eq!(lamp.name(), "Desk Lamp", "the name is stored trimmed");
    assert_eq!(lamp.quantity(), 0, "new products have no stock");
    assert!(!lamp.in_stock());
    assert_eq!(
        Product::new(sku("LAMP-01"), "   ", usd(1)),
        Err(ProductError::BlankName),
        "a blank name is rejected"
    );
}

#[test]
fn part2_name_limit_counts_characters_not_bytes() {
    let hundred = "é".repeat(100);
    assert!(
        Product::new(sku("ABCD"), &hundred, usd(1)).is_ok(),
        "100 characters fit even though they are 200 bytes"
    );
    let too_long = "é".repeat(101);
    assert_eq!(
        Product::new(sku("ABCD"), &too_long, usd(1)),
        Err(ProductError::NameTooLong { max: 100 })
    );
}

#[test]
fn part2_failed_set_name_keeps_old_name() {
    let mut item = product("ABCD", usd(1));
    item.set_name("  Gadget ").expect("valid name");
    assert_eq!(item.name(), "Gadget");
    assert_eq!(item.set_name("  "), Err(ProductError::BlankName));
    assert_eq!(item.name(), "Gadget", "a failed rename keeps the old name");
}

#[test]
fn part2_restock_rules_builder_and_setters() {
    let mut item = product("ABCD", usd(1)).with_quantity(u32::MAX - 1);
    assert_eq!(item.quantity(), u32::MAX - 1, "with_quantity seeds the stock");
    assert_eq!(item.restock(0), Err(ProductError::ZeroRestock));
    assert_eq!(item.restock(5), Err(ProductError::QuantityOverflow));
    assert_eq!(item.quantity(), u32::MAX - 1, "an overflowing restock leaves stock unchanged");
    item.restock(1).expect("fits in u32");
    assert_eq!(item.quantity(), u32::MAX);
    item.set_price(usd(2));
    assert_eq!(item.price(), usd(2));
    item.set_quantity(0);
    assert!(!item.in_stock(), "zero stock is out of stock");
}

// ---------------------------------------------------------------------------
// Part 3: Account trait, Ledger composition, overrides
// ---------------------------------------------------------------------------

#[test]
fn part3_ledger_trims_owner_and_rejects_blank() {
    let ledger = Ledger::new("  Ada  ", usd(500)).expect("valid owner");
    assert_eq!(ledger.owner(), "Ada");
    assert_eq!(ledger.balance(), usd(500));
    assert!(matches!(Ledger::new("   ", usd(0)), Err(AccountError::BlankOwner)));
}

#[test]
fn part3_deposit_and_withdraw_update_balance() {
    let mut account = CheckingAccount::open("Ada", usd(1000)).expect("valid account");
    assert_eq!(account.deposit(usd(250)), Ok(usd(1250)), "deposit returns the new balance");
    assert_eq!(account.withdraw(usd(50)), Ok(usd(1200)));
    assert_eq!(account.owner(), "Ada");
    assert_eq!(account.deposit(usd(0)), Err(AccountError::NonPositiveAmount), "zero is not a deposit");
    assert_eq!(
        account.withdraw(usd(-5)),
        Err(AccountError::NonPositiveAmount),
        "negative withdrawals are rejected"
    );
    assert_eq!(account.balance(), usd(1200), "rejected operations leave the balance unchanged");
}

#[test]
fn part3_withdraw_allows_exact_available_and_rejects_excess() {
    let mut account = CheckingAccount::open("Ada", usd(300)).expect("valid account");
    assert_eq!(
        account.withdraw(usd(301)),
        Err(AccountError::InsufficientFunds {
            requested: usd(301),
            available: usd(300),
        })
    );
    assert_eq!(account.balance(), usd(300), "the refused withdrawal changes nothing");
    assert_eq!(account.withdraw(usd(300)), Ok(usd(0)), "exactly the available amount is allowed");
}

#[test]
fn part3_overdraft_extends_available_funds() {
    let account = CheckingAccount::open("Ada", usd(100)).expect("valid account");
    assert_eq!(account.overdraft_limit(), usd(0), "no overdraft by default");
    let mut account = account.with_overdraft(usd(50)).expect("positive limit");
    assert_eq!(account.overdraft_limit(), usd(50));
    assert_eq!(account.available_funds(), Ok(usd(150)), "available is balance plus overdraft");
    assert_eq!(account.withdraw(usd(150)), Ok(usd(-50)), "the balance may go negative");
    assert_eq!(account.available_funds(), Ok(usd(0)));
    assert!(matches!(account.withdraw(usd(1)), Err(AccountError::InsufficientFunds { .. })));
}

#[test]
fn part3_overdraft_rejects_negative_and_foreign_limits() {
    let account = CheckingAccount::open("Ada", usd(100)).expect("valid account");
    assert_eq!(
        account.clone().with_overdraft(usd(-1)),
        Err(AccountError::NegativeOverdraft)
    );
    assert!(matches!(
        account.with_overdraft(eur(10)),
        Err(AccountError::Money(MoneyError::CurrencyMismatch { .. }))
    ));
}

#[test]
fn part3_currency_mismatch_on_deposit_wraps_money_error() {
    let mut account = CheckingAccount::open("Ada", usd(100)).expect("valid account");
    assert!(matches!(
        account.deposit(eur(10)),
        Err(AccountError::Money(MoneyError::CurrencyMismatch { .. }))
    ));
}

#[test]
fn part3_savings_limits_successful_withdrawals() {
    let mut account = SavingsAccount::open("Ada", usd(10_000)).expect("valid account");
    assert_eq!(account.max_withdrawals(), 3, "the default limit is 3");
    assert_eq!(account.withdraw(usd(100)), Ok(usd(9_900)));
    account.withdraw(usd(100)).expect("second withdrawal");
    account.withdraw(usd(100)).expect("third withdrawal");
    assert_eq!(account.withdrawals_this_period(), 3);
    assert_eq!(
        account.withdraw(usd(100)),
        Err(AccountError::WithdrawalLimitReached { limit: 3 })
    );
    assert_eq!(account.balance(), usd(9_700), "the refused withdrawal moves no money");
}

#[test]
fn part3_savings_deposits_and_failed_withdrawals_do_not_count() {
    let mut account = SavingsAccount::open("Ada", usd(100))
        .expect("valid account")
        .with_max_withdrawals(1);
    account.deposit(usd(50)).expect("deposit");
    assert!(matches!(
        account.withdraw(usd(1000)),
        Err(AccountError::InsufficientFunds { .. })
    ));
    assert_eq!(account.withdrawals_this_period(), 0, "failed withdrawals do not count");
    account.withdraw(usd(10)).expect("the first real withdrawal is allowed");
    assert_eq!(account.withdrawals_this_period(), 1);
}

#[test]
fn part3_savings_limit_is_checked_before_amount_and_resets() {
    let mut account = SavingsAccount::open("Ada", usd(100))
        .expect("valid account")
        .with_max_withdrawals(1);
    account.withdraw(usd(10)).expect("first withdrawal");
    assert_eq!(
        account.withdraw(usd(0)),
        Err(AccountError::WithdrawalLimitReached { limit: 1 }),
        "the period limit is checked before the amount"
    );
    account.reset_period();
    assert_eq!(account.withdrawals_this_period(), 0);
    assert_eq!(account.withdraw(usd(10)), Ok(usd(80)));
}

#[test]
fn part3_accounts_are_usable_as_trait_objects() {
    let mut accounts: Vec<Box<dyn Account>> = vec![
        Box::new(CheckingAccount::open("Ada", usd(100)).expect("valid account")),
        Box::new(SavingsAccount::open("Grace", usd(200)).expect("valid account")),
    ];
    for account in &mut accounts {
        account.deposit(usd(10)).expect("deposit");
    }
    let balances: Vec<Money> = accounts.iter().map(|account| account.balance()).collect();
    assert_eq!(balances, vec![usd(110), usd(210)]);
    let owners: Vec<&str> = accounts.iter().map(|account| account.owner()).collect();
    assert_eq!(owners, ["Ada", "Grace"]);
}

// ---------------------------------------------------------------------------
// Part 4: ShoppingCart
// ---------------------------------------------------------------------------

#[test]
fn part4_add_accumulates_quantity_and_units() {
    let mut cart = cart_usd();
    assert!(cart.is_empty(), "a new cart is empty");
    let lamp = product("LAMP-01", usd(1999));
    cart.add(&lamp, 2).expect("first add");
    cart.add(&lamp, 3).expect("same price accumulates");
    assert_eq!(cart.quantity_of(lamp.sku()), 5);
    assert!(cart.contains(lamp.sku()));
    assert_eq!(cart.units(), 5);
    assert!(!cart.is_empty());
    assert_eq!(cart.currency(), currency("USD"));
}

#[test]
fn part4_add_rejects_zero_and_foreign_currency() {
    let mut cart = cart_usd();
    let lamp = product("LAMP-01", usd(1999));
    assert_eq!(cart.add(&lamp, 0), Err(CartError::ZeroQuantity));
    let euro_mug = Product::new(sku("MUG-EUR1"), "Mug", eur(500)).expect("valid product");
    assert_eq!(
        cart.add(&euro_mug, 1),
        Err(CartError::CurrencyMismatch {
            expected: currency("USD"),
            found: currency("EUR"),
        })
    );
    assert!(cart.is_empty(), "rejected adds leave the cart unchanged");
}

#[test]
fn part4_price_change_is_rejected() {
    let mut cart = cart_usd();
    cart.add(&product("LAMP-01", usd(1999)), 1).expect("first add");
    let repriced = product("LAMP-01", usd(2499));
    assert_eq!(
        cart.add(&repriced, 1),
        Err(CartError::PriceChanged { sku: sku("LAMP-01") })
    );
    assert_eq!(cart.quantity_of(&sku("LAMP-01")), 1, "the cart is unchanged");
    assert_eq!(cart.total(), Ok(usd(1999)), "the line keeps its original price");
}

#[test]
fn part4_remove_partial_and_full_deletes_line() {
    let mut cart = cart_usd();
    let lamp = product("LAMP-01", usd(1999));
    cart.add(&lamp, 3).expect("add");
    cart.remove(lamp.sku(), 1).expect("partial removal");
    assert_eq!(cart.quantity_of(lamp.sku()), 2);
    cart.remove(lamp.sku(), 2).expect("remove the rest");
    assert!(!cart.contains(lamp.sku()), "removing every unit deletes the line");
    assert!(cart.is_empty());
}

#[test]
fn part4_remove_errors_leave_cart_unchanged() {
    let mut cart = cart_usd();
    let lamp = product("LAMP-01", usd(1999));
    cart.add(&lamp, 2).expect("add");
    assert_eq!(cart.remove(lamp.sku(), 0), Err(CartError::ZeroQuantity));
    assert_eq!(
        cart.remove(&sku("NOPE-0001"), 1),
        Err(CartError::UnknownSku { sku: sku("NOPE-0001") })
    );
    assert_eq!(
        cart.remove(lamp.sku(), 5),
        Err(CartError::InsufficientQuantity {
            sku: sku("LAMP-01"),
            requested: 5,
            available: 2,
        })
    );
    assert_eq!(cart.quantity_of(lamp.sku()), 2, "failed removals change nothing");
}

#[test]
fn part4_iteration_keeps_insertion_order() {
    let mut cart = cart_usd();
    for code in ["MUG-0001", "LAMP-01", "DESK-001"] {
        cart.add(&product(code, usd(100)), 1).expect("add");
    }
    let mut borrowed: Vec<&str> = Vec::new();
    for line in &cart {
        borrowed.push(line.sku().as_str());
    }
    assert_eq!(borrowed, ["MUG-0001", "LAMP-01", "DESK-001"], "borrowed iteration keeps order");
    let owned: Vec<String> = cart.into_iter().map(|line| line.sku().to_string()).collect();
    assert_eq!(owned, ["MUG-0001", "LAMP-01", "DESK-001"], "owned iteration keeps order");
}

#[test]
fn part4_index_returns_quantity() {
    let mut cart = cart_usd();
    cart.add(&product("LAMP-01", usd(1999)), 2).expect("add");
    assert_eq!(cart[&sku("LAMP-01")], 2);
}

#[test]
#[should_panic(expected = "no line")]
fn part4_index_panics_when_absent() {
    let cart = cart_usd();
    let _ = cart[&sku("LAMP-01")];
}

#[test]
fn part4_total_sums_subtotals_and_empty_is_zero() {
    let mut cart = cart_usd();
    assert_eq!(cart.total(), Ok(usd(0)), "an empty cart totals zero");
    cart.add(&product("LAMP-01", usd(1999)), 2).expect("add");
    cart.add(&product("MUG-0001", usd(500)), 1).expect("add");
    assert_eq!(cart.total(), Ok(usd(4498)));
}

#[test]
fn part4_try_extend_is_all_or_nothing() {
    let mut cart = cart_usd();
    let lamp = product("LAMP-01", usd(1999));
    let mug = product("MUG-0001", usd(500));
    cart.try_extend([(&lamp, 1), (&mug, 2)]).expect("the batch succeeds");
    assert_eq!(cart.units(), 3);
    let bad_batch = [(&lamp, 1), (&mug, 0)];
    assert_eq!(cart.try_extend(bad_batch), Err(CartError::ZeroQuantity));
    assert_eq!(cart.units(), 3, "a failed batch applies nothing");
    assert_eq!(cart.quantity_of(lamp.sku()), 1);
}

#[test]
fn part4_add_detects_quantity_overflow() {
    let mut cart = cart_usd();
    let lamp = product("LAMP-01", usd(1));
    cart.add(&lamp, u32::MAX).expect("the first add fits");
    assert_eq!(cart.add(&lamp, 1), Err(CartError::QuantityOverflow));
    assert_eq!(cart.quantity_of(lamp.sku()), u32::MAX, "the line is unchanged");
}

// ---------------------------------------------------------------------------
// Part 5: Employee, TryFrom, FromStr, serde
// ---------------------------------------------------------------------------

fn record(email: &str, salary: &str) -> EmployeeRecord {
    EmployeeRecord {
        employee_id: 7,
        name: " Grace Hopper ".to_owned(),
        email: email.to_owned(),
        department: "Navy".to_owned(),
        salary: salary.to_owned(),
        currency: "USD".to_owned(),
        skills: Vec::new(),
    }
}

#[test]
fn part5_email_normalises_and_rejects_malformed() {
    assert_eq!(
        Email::parse("  Ada@Example.COM ").expect("valid").as_str(),
        "ada@example.com",
        "emails are trimmed and lowercased"
    );
    for bad in ["ada.example.com", "a@b@c.com", "ada @example.com", "@example.com", "ada@"] {
        assert!(
            matches!(Email::parse(bad), Err(EmployeeError::InvalidEmail(_))),
            "{bad:?} should be rejected"
        );
    }
}

#[test]
fn part5_record_is_validated_and_trimmed() {
    let employee = Employee::try_from(record("Grace@Navy.mil", "85000.00")).expect("valid record");
    assert_eq!(employee.employee_id(), 7);
    assert_eq!(employee.name(), "Grace Hopper", "the name is trimmed");
    assert_eq!(employee.department(), "Navy");
    assert_eq!(employee.email().as_str(), "grace@navy.mil");
    assert_eq!(employee.salary(), Money::new(8_500_000, currency("USD")));
}

#[test]
fn part5_record_rejects_invalid_fields() {
    let mut zero_id = record("a@b.io", "1.00");
    zero_id.employee_id = 0;
    assert!(matches!(
        Employee::try_from(zero_id),
        Err(EmployeeError::Invalid { field: "employee_id", .. })
    ));

    let mut blank_name = record("a@b.io", "1.00");
    blank_name.name = "   ".to_owned();
    assert!(matches!(
        Employee::try_from(blank_name),
        Err(EmployeeError::Invalid { field: "name", .. })
    ));

    assert!(
        matches!(
            Employee::try_from(record("a@b.io", "-1.00")),
            Err(EmployeeError::Invalid { field: "salary", .. })
        ),
        "a negative salary is invalid"
    );
    assert!(matches!(
        Employee::try_from(record("not-an-email", "1.00")),
        Err(EmployeeError::InvalidEmail(_))
    ));
}

#[test]
fn part5_skills_are_normalised_and_deduplicated() {
    let mut raw = record("a@b.io", "1.00");
    raw.skills = vec![" Rust ".into(), String::new(), "SQL".into(), "rust".into(), "  ".into()];
    let employee = Employee::try_from(raw).expect("valid record");
    assert_eq!(
        employee.skills(),
        ["rust", "sql"],
        "skills are trimmed and lowercased; empties and duplicates are dropped"
    );
}

#[test]
fn part5_from_str_parses_quoted_csv_row() {
    let employee: Employee = "42,\"Lovelace, Ada\",ada@example.com,Engineering,85000.00,USD,Rust;SQL;rust"
        .parse()
        .expect("valid row");
    assert_eq!(employee.name(), "Lovelace, Ada", "a quoted comma stays inside one field");
    assert_eq!(employee.skills(), ["rust", "sql"]);
    assert_eq!(employee.salary(), Money::new(8_500_000, currency("USD")));

    let bare: Employee = "43,Bob,bob@example.com,Ops,1.00,EUR,".parse().expect("empty skills column");
    assert!(bare.skills().is_empty());
}

#[test]
fn part5_from_str_rejects_wrong_column_count() {
    assert!(matches!(
        "42,Ada,ada@example.com,Engineering,85000.00,USD".parse::<Employee>(),
        Err(EmployeeError::ColumnCount(6))
    ));
}

#[test]
fn part5_serde_json_goes_through_validation() {
    let good = r#"{"employee_id":9,"name":"Ada","email":"Ada@Example.com","department":"R&D","salary":"1.00","currency":"GBP"}"#;
    let employee: Employee = serde_json::from_str(good).expect("valid JSON");
    assert_eq!(employee.email().as_str(), "ada@example.com");
    assert!(employee.skills().is_empty(), "missing skills default to empty");
    let bad = good.replace("\"employee_id\":9", "\"employee_id\":0");
    assert!(
        serde_json::from_str::<Employee>(&bad).is_err(),
        "JSON cannot bypass validation"
    );
}

// ---------------------------------------------------------------------------
// Part 6: Notifier trait objects, transports, dispatch
// ---------------------------------------------------------------------------

#[test]
fn part6_email_channel_delivers_to_transport() {
    let sent = RefCell::new(Vec::<String>::new());
    let email = EmailNotifier::new(|to: &str, body: &str| -> Result<(), BoxError> {
        sent.borrow_mut().push(format!("{to}:{body}"));
        Ok(())
    });
    assert_eq!(email.channel_name(), "email");
    assert_eq!(email.describe(), "email notifier", "the default description");
    email.send("ada@example.com", "Hello").expect("valid send");
    assert_eq!(sent.borrow().as_slice(), ["ada@example.com:Hello"]);
}

#[test]
fn part6_recipient_is_checked_before_transport() {
    let calls = Cell::new(0_u32);
    let email = EmailNotifier::new(|_: &str, _: &str| -> Result<(), BoxError> {
        calls.set(calls.get() + 1);
        Ok(())
    });
    for bad in ["ada.example.com", "ada@", "a b@example.com", "a@b@c"] {
        assert!(
            matches!(
                email.send(bad, "hi"),
                Err(NotificationError::InvalidRecipient { channel: "email", .. })
            ),
            "{bad:?} should be an invalid email recipient"
        );
    }
    assert!(
        matches!(email.send("ada@example.com", " "), Err(NotificationError::BlankMessage)),
        "a whitespace-only message is blank"
    );
    assert_eq!(calls.get(), 0, "the transport is never called for invalid input");
}

#[test]
fn part6_transport_failure_is_exposed_as_source() {
    let email = EmailNotifier::new(|_: &str, _: &str| -> Result<(), BoxError> {
        Err("smtp down".into())
    });
    let err = email.send("ada@example.com", "hi").expect_err("the transport fails");
    assert!(matches!(err, NotificationError::Transport(_)));
    let source = std::error::Error::source(&err).expect("the transport error is the source");
    assert_eq!(source.to_string(), "smtp down");
}

#[test]
fn part6_sms_recipients_follow_e164_rules() {
    let sms = SmsNotifier::new(|_: &str, _: &str| -> Result<(), BoxError> { Ok(()) });
    assert_eq!(sms.channel_name(), "sms");
    for ok in ["+4915123456", "+1234567"] {
        assert!(sms.send(ok, "hi").is_ok(), "{ok:?} should be accepted");
    }
    for bad in ["015123456", "+0123456789", "+123456", "+1234567890123456", "+12a4567890", ""] {
        assert!(
            matches!(
                sms.send(bad, "hi"),
                Err(NotificationError::InvalidRecipient { channel: "sms", .. })
            ),
            "{bad:?} should be rejected"
        );
    }
}

#[test]
fn part6_sms_length_counts_characters() {
    let sms = SmsNotifier::new(|_: &str, _: &str| -> Result<(), BoxError> { Ok(()) });
    let at_limit = "é".repeat(SMS_MAX_CHARS);
    assert!(
        sms.send("+1234567", &at_limit).is_ok(),
        "160 characters fit even though they are 320 bytes"
    );
    let over = "é".repeat(SMS_MAX_CHARS + 1);
    assert!(matches!(
        sms.send("+1234567", &over),
        Err(NotificationError::MessageTooLong { len: 161, max: 160 })
    ));
}

#[test]
fn part6_webhook_requires_https_endpoint() {
    let post = |_: &str, _: &[u8]| -> Result<u16, BoxError> { Ok(200) };
    assert!(matches!(
        WebhookNotifier::new("http://hooks.example.com/x", post),
        Err(NotificationError::InsecureEndpoint(_))
    ));
    assert!(WebhookNotifier::new("https://hooks.example.com/x", post).is_ok());
}

#[test]
fn part6_webhook_posts_json_and_checks_status() {
    let seen = RefCell::new(Vec::<(String, String)>::new());
    let hook = WebhookNotifier::new(
        "https://hooks.example.com/alerts",
        |endpoint: &str, body: &[u8]| -> Result<u16, BoxError> {
            seen.borrow_mut()
                .push((endpoint.to_owned(), String::from_utf8(body.to_vec())?));
            Ok(204)
        },
    )
    .expect("https endpoint");
    assert_eq!(hook.channel_name(), "webhook");

    hook.send("ops-team", "disk full").expect("204 is success");
    let (endpoint, body) = seen.borrow()[0].clone();
    assert_eq!(endpoint, "https://hooks.example.com/alerts");
    let json: serde_json::Value = serde_json::from_str(&body).expect("the body is JSON");
    assert_eq!(json["message"], "disk full");
    assert_eq!(json["recipient"], "ops-team");

    assert!(
        matches!(hook.send("   ", "disk full"), Err(NotificationError::BlankRecipient)),
        "a blank recipient is rejected"
    );

    let failing = WebhookNotifier::new(
        "https://hooks.example.com/alerts",
        |_: &str, _: &[u8]| -> Result<u16, BoxError> { Ok(500) },
    )
    .expect("https endpoint");
    assert!(matches!(
        failing.send("ops-team", "disk full"),
        Err(NotificationError::HttpStatus(500))
    ));
}

#[test]
fn part6_dispatcher_rejects_duplicate_channel_names() {
    let duplicates = NotificationDispatcher::new(vec![email_channel(), email_channel()]);
    assert!(matches!(
        duplicates,
        Err(NotificationError::DuplicateChannel(ref name)) if name == "email"
    ));
    let empty = NotificationDispatcher::new(Vec::<Box<dyn Notifier>>::new()).expect("empty is fine");
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
}

#[test]
fn part6_dispatch_continues_after_failures_and_keeps_request_order() {
    let log: Arc<Mutex<Vec<String>>> = Arc::default();
    let email_log = Arc::clone(&log);
    let sms_log = Arc::clone(&log);
    let email = EmailNotifier::new(move |to: &str, _: &str| -> Result<(), BoxError> {
        email_log.lock().expect("log lock").push(format!("email:{to}"));
        if to.starts_with("bounce") {
            Err("mailbox full".into())
        } else {
            Ok(())
        }
    });
    let sms = SmsNotifier::new(move |to: &str, _: &str| -> Result<(), BoxError> {
        sms_log.lock().expect("log lock").push(format!("sms:{to}"));
        Ok(())
    });
    let channels: Vec<Box<dyn Notifier>> = vec![Box::new(email), Box::new(sms)];
    let dispatcher = NotificationDispatcher::new(channels).expect("unique channels");
    assert_eq!(dispatcher.len(), 2);
    assert_eq!(dispatcher.channels().collect::<Vec<_>>(), ["email", "sms"]);

    let results = dispatcher
        .dispatch(
            "Hi",
            &[
                ("email", "bounce@example.com"),
                ("sms", "+1234567"),
                ("email", "ada@example.com"),
            ],
        )
        .expect("every channel is known");
    let labels: Vec<&str> = results.iter().map(|(channel, _)| *channel).collect();
    assert_eq!(labels, ["email", "sms", "email"], "results follow the request order");
    assert!(matches!(results[0].1, Err(NotificationError::Transport(_))), "the failure is recorded");
    assert!(results[1].1.is_ok());
    assert!(results[2].1.is_ok(), "the loop continues after a failure");
    assert_eq!(
        *log.lock().expect("log lock"),
        ["email:bounce@example.com", "sms:+1234567", "email:ada@example.com"]
    );

    let unknown = dispatcher.dispatch("Hi", &[("email", "ada@example.com"), ("pager", "x")]);
    assert!(matches!(
        unknown,
        Err(NotificationError::UnknownChannel(ref name)) if name == "pager"
    ));
    assert_eq!(
        log.lock().expect("log lock").len(),
        3,
        "an unknown channel is detected before anything is sent"
    );
}

// ---------------------------------------------------------------------------
// Part 7: Version
// ---------------------------------------------------------------------------

#[test]
fn part7_parse_accepts_prefix_and_round_trips() {
    let full = v("v1.2.3-rc.1+build.5");
    assert_eq!((full.major(), full.minor(), full.patch()), (1, 2, 3));
    assert_eq!(
        full.prerelease(),
        [PreIdentifier::Alphanumeric("rc".to_owned()), PreIdentifier::Numeric(1)]
    );
    assert_eq!(full.build(), ["build", "5"]);
    assert!(full.is_prerelease());
    assert!(!v("1.0.0").is_prerelease());
    assert_eq!(full.to_string(), "1.2.3-rc.1+build.5", "display keeps the build metadata");
    assert_eq!("1.0.0".parse::<Version>().expect("valid").to_string(), "1.0.0");
    assert_eq!(Version::new(4, 5, 6).to_string(), "4.5.6", "const constructor");
}

#[test]
fn part7_rejects_leading_zeros() {
    assert!(
        matches!(Version::parse("01.2.3"), Err(VersionError::LeadingZero(_))),
        "core numbers have no leading zeros"
    );
    assert!(
        matches!(Version::parse("1.0.0-01"), Err(VersionError::LeadingZero(_))),
        "numeric pre-release identifiers have no leading zeros"
    );
    assert!(
        Version::parse("1.0.0+01").is_ok(),
        "numeric build identifiers may have leading zeros"
    );
}

#[test]
fn part7_rejects_empty_and_invalid_identifiers() {
    assert!(matches!(Version::parse("1.0.0-"), Err(VersionError::EmptyIdentifier(_))));
    assert!(matches!(Version::parse("1.0.0-rc..1"), Err(VersionError::EmptyIdentifier(_))));
    assert!(matches!(
        Version::parse("1.0.0-rc_1"),
        Err(VersionError::InvalidCharacter('_'))
    ));
}

#[test]
fn part7_rejects_overflow_and_malformed_text() {
    assert!(matches!(
        Version::parse("18446744073709551616.0.0"),
        Err(VersionError::Overflow)
    ));
    assert!(matches!(Version::parse("1.2"), Err(VersionError::Malformed(_))));
    assert!(matches!(Version::parse("1.2.3.4"), Err(VersionError::Malformed(_))));
    assert!(matches!(Version::parse(""), Err(VersionError::Malformed(_))));
}

#[test]
fn part7_precedence_follows_semver() {
    let ordered = [
        "1.0.0-alpha",
        "1.0.0-alpha.1",
        "1.0.0-alpha.beta",
        "1.0.0-beta",
        "1.0.0-beta.2",
        "1.0.0-beta.11",
        "1.0.0-rc.1",
        "1.0.0",
    ];
    let versions: Vec<Version> = ordered.into_iter().map(v).collect();
    for pair in versions.windows(2) {
        assert!(pair[0] < pair[1], "{} should sort before {}", pair[0], pair[1]);
    }
    assert!(v("1.0.0-2") < v("1.0.0-10"), "numeric identifiers compare numerically");
    assert!(v("1.0.0-1") < v("1.0.0-a"), "numeric identifiers rank below alphanumeric");
    assert!(v("1.0.0-alpha") < v("1.0.1-alpha"), "core numbers dominate the prerelease");
}

#[test]
fn part7_build_metadata_is_ignored_by_eq_ord_and_hash() {
    let a = v("1.0.0+a");
    let b = v("1.0.0+b");
    assert_eq!(a, b, "build metadata does not affect equality");
    assert_eq!(a.cmp(&b), Ordering::Equal, "nor ordering");
    assert_eq!(hash_of(&a), hash_of(&b), "equal values must hash equally");
    assert_ne!(a.to_string(), b.to_string(), "display still shows the metadata");
}

#[test]
fn part7_bump_resets_lower_parts_and_clears_suffixes() {
    let base = v("1.2.3-rc.1+build.5");
    assert_eq!(base.bump(Part::Patch).expect("bump").to_string(), "1.2.4");
    assert_eq!(base.bump(Part::Minor).expect("bump").to_string(), "1.3.0");
    assert_eq!(base.bump(Part::Major).expect("bump").to_string(), "2.0.0");
    assert!(
        matches!(
            v("18446744073709551615.0.0").bump(Part::Major),
            Err(VersionError::Overflow)
        ),
        "a bump past u64::MAX overflows"
    );
}
