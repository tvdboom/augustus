use super::*;

#[test]
fn income_and_expenses_share_unsigned_zero_and_expenses_show_a_minus() {
    let muted = egui::Color32::GRAY;
    for value in [0.0, -0.0, 0.1, -0.1, 0.9, -0.9] {
        assert_eq!(flow_number(value, true), "0");
        assert_eq!(flow_number(value, false), "0");
        assert_eq!(flow_color(value, true, muted), muted);
        assert_eq!(flow_color(value, false, muted), muted);
    }
    assert_eq!(flow_number(1.0, false), "-1");
    assert_eq!(flow_number(5.0, false), "-5");
    assert_eq!(flow_number(1_550.0, false), "-1.6k");
    assert_eq!(flow_number(46.0, true), "46");
}
