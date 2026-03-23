use super::{error::StructName, tree_repr::Change};
use crate::{Collision, Tree, hash_cache::HashCache};
use element::Element;

impl<const DEPTH: usize, V, C> Tree<DEPTH, V, C>
where
    C: HashCache,
{
    /// Insert into the tree and btreemap at the same time, without updating the hash
    pub(crate) fn insert_without_hashing(
        &mut self,
        insert_entries: Vec<(Element, V)>,
        remove_entries: &[Element],
    ) -> Result<(), Collision> {
        let insert_elements = insert_entries
            .iter()
            .map(|(e, _)| e)
            .copied()
            .collect::<Vec<_>>();

        if insert_elements.is_empty() && remove_entries.is_empty() {
            return Ok(());
        }

        if insert_elements
            .iter()
            .chain(remove_entries.iter())
            .any(|e| e == &Element::NULL_HASH)
        {
            return Err(Collision {
                inserted: Element::NULL_HASH,
                in_tree: Element::NULL_HASH,
                depth: DEPTH,
                struct_name: StructName::Tree,
            });
        }

        if let Some(element) = insert_elements
            .iter()
            .find(|e| self.entries.contains_key(e))
        {
            return Err(Collision {
                in_tree: *element,
                inserted: *element,
                depth: DEPTH,
                struct_name: StructName::Tree,
            });
        }

        if let Some(element) = remove_entries
            .iter()
            .find(|e| !self.entries.contains_key(e))
        {
            todo!(
                "return error that we can't remove this element because it's not in tree {element:?}"
            );
        }

        let mut elements_and_bits = Vec::with_capacity(insert_entries.len() + remove_entries.len());
        for (change, element) in insert_entries
            .iter()
            .map(|(e, _)| (Change::Insert, e))
            .chain(remove_entries.iter().map(|e| (Change::Remove, e)))
        {
            // if the tree has depth n, we need n-1 bits, since there are n-1 left/right decisions
            elements_and_bits.push(((change, *element), element.lsb(DEPTH - 1).to_bitvec()));
        }
        elements_and_bits.sort_unstable_by(|(_, a_bits), (_, b_bits)| a_bits.cmp(b_bits));

        let (elements, bits): (Vec<_>, Vec<_>) = elements_and_bits.into_iter().unzip();

        let result = self
            .tree
            .insert_without_hashing::<DEPTH>(&elements, &bits, 0)?;

        match result {
            true => {
                for (element, value) in insert_entries {
                    self.entries.insert(element, value);
                }

                for element in remove_entries {
                    self.entries.remove(element);
                }
            }
            false => unreachable!(
                "we check if the tree contains the element earlier, so this should be impossible"
            ),
        }

        Ok(())
    }
}
