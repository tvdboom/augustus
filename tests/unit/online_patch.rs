use super::*;
use serde_json::json;

#[test]
fn disjoint_players_merge_but_conflicting_wallets_are_atomic() {
    let base = json!({"campaign":{"players":[{"coin":100},{"coin":200}],"units":[]}});
    let mut host = base.clone();
    host["campaign"]["players"][0]["coin"] = json!(80);
    let mut guest = base.clone();
    guest["campaign"]["players"][1]["coin"] = json!(150);
    let merged = apply(&host, &changes(&base, &guest), true).unwrap();
    assert_eq!(merged["campaign"]["players"][0]["coin"], 80);
    assert_eq!(merged["campaign"]["players"][1]["coin"], 150);
    assert!(apply(&merged, &changes(&base, &guest), true).is_err());
    assert_eq!(base["campaign"]["players"][1]["coin"], 200);
}

#[test]
fn growing_arrays_and_new_object_keys_replay_without_old_values() {
    let before = json!({"campaign":{"units":[],"access":{}}});
    let after = json!({"campaign":{"units":[{"id":1}],"access":{"guest":true}}});
    let deltas = changes(&before, &after);
    assert_eq!(apply(&before, &deltas, true).unwrap(), after);
    let replay: Vec<_> = deltas
        .into_iter()
        .map(|mut d| {
            d.before = None;
            d
        })
        .collect();
    assert_eq!(apply(&before, &replay, false).unwrap(), after);
    assert!(!serde_json::to_string(&replay).unwrap().contains("before"));
}

#[test]
fn invalid_paths_and_late_conflicts_never_partially_apply() {
    let base = json!({"campaign":{"coin":100,"influence":20}});
    let changes = vec![
        Change {
            path: vec!["campaign".into(), "coin".into()],
            before: Some(json!(100)),
            after: json!(50),
        },
        Change {
            path: vec!["campaign".into(), "influence".into()],
            before: Some(json!(99)),
            after: json!(10),
        },
    ];
    assert!(apply(&base, &changes, true).is_err());
    assert_eq!(base["campaign"]["coin"], 100);
}
