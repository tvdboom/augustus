use super::*;

#[test]
fn income_and_expenses_show_fractional_amounts_and_unsigned_zero() {
    let muted = egui::Color32::GRAY;
    for value in [0.0, -0.0] {
        assert_eq!(flow_number(value, true), "0");
        assert_eq!(flow_number(value, false), "0");
        assert_eq!(flow_color(value, true, muted), muted);
        assert_eq!(flow_color(value, false, muted), muted);
    }
    assert_eq!(flow_number(0.6, false), "-0.6");
    assert_eq!(flow_number(0.01, false), "-0.01");
    assert_eq!(flow_number(27.6, false), "-27.6");
    assert_eq!(flow_number(0.6, true), "0.6");
    assert_eq!(flow_color(0.6, false, muted), hud_delta_color(-1.0));
    assert_eq!(flow_number(1.0, false), "-1");
    assert_eq!(flow_number(5.0, false), "-5");
    assert_eq!(flow_number(1_550.0, false), "-1.6k");
    assert_eq!(flow_number(46.0, true), "46");
}
