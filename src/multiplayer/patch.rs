//! Atomic, preconditioned state deltas. Replays omit old values to reduce egress.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A replacement at an existing JSON path, with an optional compare-and-set value.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Change {
    /// Object keys or array indexes from the snapshot root.
    pub path: Vec<String>,
    /// Value expected by a writer; omitted in durable event replay.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<Value>,
    /// Replacement value.
    pub after: Value,
}

/// Computes deterministic replacements, replacing arrays when their length changes.
pub fn changes(before: &Value, after: &Value) -> Vec<Change> {
    fn visit(before: &Value, after: &Value, path: &mut Vec<String>, out: &mut Vec<Change>) {
        if before == after {
            return;
        }
        let start = out.len();
        match (before, after) {
            (Value::Object(a), Value::Object(b)) if a.keys().eq(b.keys()) => {
                for (key, old) in a {
                    path.push(key.clone());
                    visit(old, &b[key], path, out);
                    path.pop();
                }
            },
            (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
                for (index, (old, new)) in a.iter().zip(b).enumerate() {
                    path.push(index.to_string());
                    visit(old, new, path, out);
                    path.pop();
                }
            },
            _ => out.push(Change {
                path: path.clone(),
                before: Some(before.clone()),
                after: after.clone(),
            }),
        }
        // Many changing numeric fields can cost more in repeated paths than one
        // compact parent replacement. Compare actual wire sizes, including replay.
        if out.len() > start + 1 {
            let parent = Change {
                path: path.clone(),
                before: Some(before.clone()),
                after: after.clone(),
            };
            let child_cost = out[start..]
                .iter()
                .map(|c| serde_json::to_vec(c).map(|v| v.len()).unwrap_or(usize::MAX / 20000))
                .sum::<usize>();
            let parent_cost = serde_json::to_vec(&parent).map(|v| v.len()).unwrap_or(usize::MAX);
            // Keep the snapshot and campaign roots stable for authorization and merging.
            if path.len() >= 2 && parent_cost < child_cost {
                out.truncate(start);
                out.push(parent);
            }
        }
    }
    let mut out = Vec::new();
    visit(before, after, &mut Vec::new(), &mut out);
    out
}

/// Applies all replacements to a copy, rejecting conflicts without partial mutation.
pub fn apply(state: &Value, changes: &[Change], check: bool) -> Result<Value, String> {
    let mut next = state.clone();
    for change in changes {
        let mut target = &mut next;
        for key in &change.path {
            target = match target {
                Value::Object(map) => map.get_mut(key),
                Value::Array(array) => key.parse::<usize>().ok().and_then(|i| array.get_mut(i)),
                _ => None,
            }
            .ok_or_else(|| "The game changed; retry your action.".to_string())?;
        }
        if check && change.before.as_ref() != Some(&*target) {
            return Err("The game changed; retry your action.".into());
        }
        *target = change.after.clone();
    }
    Ok(next)
}

/// Serializes maps with structured keys as stable arrays rather than JSON object keys.
pub mod pairs {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;
    /// Encode all keys without lossy string conversion.
    pub fn serialize<K: Serialize, V: Serialize, S: Serializer>(
        map: &BTreeMap<K, V>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        map.iter().collect::<Vec<_>>().serialize(s)
    }
    /// Decode keys into their original ordered map.
    pub fn deserialize<
        'de,
        K: Deserialize<'de> + Ord,
        V: Deserialize<'de>,
        D: Deserializer<'de>,
    >(
        d: D,
    ) -> Result<BTreeMap<K, V>, D::Error> {
        Vec::<(K, V)>::deserialize(d).map(|pairs| pairs.into_iter().collect())
    }
}

/// Encode the two coalitions' structured-key maps using the same pair format.
pub mod pair_maps {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    /// Encode both maps without converting structured owner keys to strings.
    pub fn serialize<K: Serialize, V: Serialize, S: Serializer>(
        maps: &[BTreeMap<K, V>; 2],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        maps.each_ref().map(|map| map.iter().collect::<Vec<_>>()).serialize(serializer)
    }

    /// Restore each coalition's ordered map from its stable pair array.
    pub fn deserialize<
        'de,
        K: Deserialize<'de> + Ord,
        V: Deserialize<'de>,
        D: Deserializer<'de>,
    >(
        deserializer: D,
    ) -> Result<[BTreeMap<K, V>; 2], D::Error> {
        <[Vec<(K, V)>; 2]>::deserialize(deserializer)
            .map(|maps| maps.map(|pairs| pairs.into_iter().collect()))
    }
}

#[cfg(test)]
#[path = "../../tests/unit/online_patch.rs"]
mod tests;
