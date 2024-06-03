use crate::{hash_cache::HashCache, Collision, Element, Tree};

use super::error::StructName;

impl<const DEPTH: usize, V, C> Tree<DEPTH, V, C>
where
    C: HashCache,
{
    /// Insert into the tree and btreemap at the same time, without updating the hash
    pub(crate) fn insert_without_hashing(
        &mut self,
        element: Element,
        value: V,
    ) -> Result<(), Collision> {
        if element == Element::NULL_HASH {
            return Err(Collision {
                inserted: Element::NULL_HASH,
                in_tree: Element::NULL_HASH,
                depth: DEPTH,
                struct_name: StructName::Tree,
            });
        }

        if self.entries.contains_key(&element) {
            return Err(Collision {
                in_tree: element,
                inserted: element,
                depth: DEPTH,
                struct_name: StructName::Tree,
            });
        }

        // if the tree has depth n, we need n-1 bits, since there are n-1 left/right decisions
        let bits = element.lsb(DEPTH - 1);
        let result = self.tree.insert_without_hashing::<DEPTH>(element, &bits)?;

        match result {
            true => self.entries.insert(element, value),
            false => unreachable!(
                "we check if the tree contains the element earlier, so this should be impossible"
            ),
        };

        Ok(())
    }
}
