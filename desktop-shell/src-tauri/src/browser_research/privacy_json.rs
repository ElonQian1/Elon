//! Validate JSON key identity before serde_json::Value can discard duplicate branches.
use serde::de::{DeserializeSeed, Deserializer, Error, MapAccess, SeqAccess, Visitor};
use std::{cell::Cell, collections::HashSet, fmt};

const MAX_CONTAINERS: usize = 64;
struct Seed<'a> {
    depth: usize,
    issue: &'a Cell<Option<&'static str>>,
}
impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}
impl Seed<'_> {
    fn container<E: Error>(&self) -> Result<(), E> {
        if self.depth >= MAX_CONTAINERS {
            self.issue.set(Some("json_depth_limit_not_captured"));
            return Err(E::custom("json_depth_limit_not_captured"));
        }
        Ok(())
    }
    fn child(&self) -> Seed<'_> {
        Seed {
            depth: self.depth + 1,
            issue: self.issue,
        }
    }
}
impl<'de> Visitor<'de> for Seed<'_> {
    type Value = ();
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON value")
    }
    fn visit_bool<E: Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E: Error>(self, _: &str) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<(), A::Error> {
        self.container()?;
        while sequence.next_element_seed(self.child())?.is_some() {}
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        self.container()?;
        let mut keys = HashSet::<String>::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key) {
                self.issue.set(Some("duplicate_json_keys_not_captured"));
                return Err(A::Error::custom("duplicate_json_keys_not_captured"));
            }
            map.next_value_seed(self.child())?;
        }
        Ok(())
    }
}

/// false means non-JSON text. Ambiguous/deep JSON returns a fixed capture gap.
/// The caller must enforce BODY_LIMIT before calling this bounded-depth visitor.
pub(super) fn is_unambiguous_json(body: &str) -> Result<bool, &'static str> {
    let issue = Cell::new(None);
    let mut parser = serde_json::Deserializer::from_str(body);
    let parsed = Seed {
        depth: 0,
        issue: &issue,
    }
    .deserialize(&mut parser)
    .is_ok();
    if let Some(code) = issue.get() {
        return Err(code);
    }
    Ok(parsed && parser.end().is_ok())
}
