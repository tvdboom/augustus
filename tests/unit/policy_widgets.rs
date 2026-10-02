use super::coin_decimal;

#[test]
fn small_policy_costs_keep_their_cents() {
    assert_eq!(coin_decimal(0.0), "0");
    assert_eq!(coin_decimal(0.01), "0.01");
    assert_eq!(coin_decimal(0.02), "0.02");
    assert_eq!(coin_decimal(0.6), "0.6");
}
