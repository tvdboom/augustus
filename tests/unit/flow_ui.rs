use super::*;

#[test]
fn income_and_expenses_show_rounded_integers_and_unsigned_zero() {
    let muted = egui::Color32::GRAY;
    for value in [0.0, -0.0, 0.01, 0.49] {
        assert_eq!(flow_number(value, true), "0");
        assert_eq!(flow_number(value, false), "0");
        assert_eq!(flow_color(value, true, muted), muted);
        assert_eq!(flow_color(value, false, muted), muted);
    }
    assert_eq!(flow_number(0.6, false), "-1");
    assert_eq!(flow_number(8.8, false), "-9");
    assert_eq!(flow_number(76.5, false), "-77");
    assert_eq!(flow_number(55.7, false), "-56");
    assert_eq!(flow_number(27.6, false), "-28");
    assert_eq!(flow_number(0.6, true), "1");
    assert_eq!(flow_color(0.6, false, muted), hud_delta_color(-1.0));
    assert_eq!(flow_number(1.0, false), "-1");
    assert_eq!(flow_number(5.0, false), "-5");
    assert_eq!(flow_number(1_550.0, false), "-1550");
    assert_eq!(flow_number(1_250_000.6, true), "1250001");
    assert_eq!(flow_number(46.0, true), "46");
}
