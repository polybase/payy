use crate::{Batch, Collision, CollisionError, tree};

impl<const DEPTH: usize, V> Batch<DEPTH, V> {
    /// Merge `other` into `self`
    ///
    /// If there are no colliding [`Element`]s, the resulting [`Batch`] will contain all the
    /// entries from both [`Batch`]es.
    ///
    /// If there are collisions, they will be returned in a [`Vec`]
    ///
    /// [`Element`]: element::Element
    pub fn merge(mut self, other: Self) -> Result<Self, CollisionError> {
        let colliding_lsbs = self.lsbs.iter().filter(|lsb| other.lsbs.contains(lsb));

        // avoid allocating a vec if we don't need to
        if colliding_lsbs.clone().count() != 0 {
            let collisions = colliding_lsbs
                .map(|lsb| {
                    // unwraps are fine beacuse any lsb here is in both
                    let element_in_self = self.find_element_with_lsb(*lsb).unwrap();
                    let element_in_other = other.find_element_with_lsb(*lsb).unwrap();

                    Collision {
                        in_tree: element_in_self,
                        inserted: element_in_other,
                        depth: DEPTH,
                        struct_name: tree::StructName::Batch,
                    }
                })
                .collect();

            return Err(CollisionError { collisions });
        }

        self.entries.extend(other.entries);
        self.lsbs.extend(other.lsbs);

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use element::Element;
    use proptest::prop_assert_eq;
    use test_strategy::proptest;

    use crate::batch;

    use super::*;

    #[test]
    fn can_merge_batches() {
        let a = batch! {
            1 => "hello",
            2 => "world",
        };

        let b = batch! {
            3 => "foo",
            4 => "bar",
        };

        let c: Batch<64, &str> = a.merge(b).unwrap();

        let expected: Batch<64, &str> = batch! {
            1 => "hello",
            2 => "world",
            3 => "foo",
            4 => "bar",
        };

        let mut entries: Vec<_> = c.insert_entries().to_vec();
        let mut expected_entries: Vec<_> = expected.insert_entries().to_vec();

        entries.sort_by_key(|tuple| tuple.0);
        expected_entries.sort_by_key(|tuple| tuple.0);

        assert_eq!(entries, expected_entries);
    }

    #[proptest]
    fn merge_batches(batch1: Batch<64, Element>, batch2: Batch<64, Element>) {
        let expected_elements: Vec<_> = batch1
            .insert_elements()
            .chain(batch2.insert_elements())
            .collect();

        let result = batch1.merge(batch2);
        proptest::prop_assume!(result.is_ok());
        let merged = result.unwrap();

        let elements: Vec<_> = merged.insert_elements().collect();

        prop_assert_eq!(elements, expected_elements);
    }
}
