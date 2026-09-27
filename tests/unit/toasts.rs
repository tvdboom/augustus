use super::*;

fn resource(amount: f64, monthly_delta: f64) -> HudResource {
    HudResource {
        amount,
        monthly_delta,
    }
}

#[test]
fn happiness_warnings_use_strict_class_thresholds() {
    let mut happiness = [resource(50.0, 0.0); 4];
    let resources = [resource(100.0, 1.0); 7];
    happiness[0].amount = 40.0;
    happiness[1].amount = 30.0;
    happiness[2].amount = 20.0;
    happiness[3].amount = 10.0;
    assert_eq!(warning_conditions(resources, happiness), [false; 6]);
    for class in 0..4 {
        happiness[class].amount -= 0.1;
    }
    assert_eq!(warning_conditions(resources, happiness), [true, true, true, true, false, false]);
}

#[test]
fn shortages_use_three_month_forecast_and_require_nonpositive_net() {
    let happiness = [resource(50.0, 0.0); 4];
    let mut resources = [resource(100.0, 1.0); 7];
    resources[0] = resource(30.0, -10.0);
    resources[3] = resource(24.0, 0.0);
    assert_eq!(warning_conditions(resources, happiness)[4..], [true, true]);
    resources[0].amount = 30.1;
    resources[3].monthly_delta = 1.0;
    assert_eq!(warning_conditions(resources, happiness)[4..], [false, false]);
}

#[test]
fn warning_only_repeats_after_recovery() {
    let mut watch = WarningWatch::default();
    let mut toasts = ToastQueue::default();
    let mut resources = [resource(100.0, 1.0); 7];
    let happiness = [resource(50.0, 0.0); 4];
    resources[0] = resource(9.0, 0.0);
    watch.observe(0, resources, happiness, &mut toasts);
    assert_eq!(toasts.0.len(), 2); // Welcome info and food warning.
    watch.observe(0, resources, happiness, &mut toasts);
    assert_eq!(toasts.0.len(), 2);
    resources[0].amount = 20.0;
    watch.observe(0, resources, happiness, &mut toasts);
    resources[0].amount = 9.0;
    watch.observe(0, resources, happiness, &mut toasts);
    assert_eq!(toasts.0.len(), 3);
}

#[test]
fn queued_toasts_request_one_sound_per_severity() {
    let mut toasts = ToastQueue::default();
    toasts.push(Toast::warning("First warning"));
    toasts.push(Toast::warning("Second warning"));
    toasts.push(Toast::info("Information"));
    toasts.push(Toast::error("Failure"));
    assert_eq!(toasts.1, [true, true, true]);
    toasts.clear();
    assert_eq!(toasts.1, [false; 3]);
}
