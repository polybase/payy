use element::Element;

/// An error indicating at least one collision occurred when trying to insert a value into the tree
///
/// ```rust
/// # use smirk::*;
/// # use element::Element;
/// let mut tree: Tree<64, _>  = smirk! { 1 };
///
/// let colliding_element = Element::ONE + (Element::ONE << 100);
/// let error = tree.insert(colliding_element, ()).unwrap_err();
/// let collision = &error.collisions()[0];
///
/// // the value that was already in the tree
/// assert_eq!(collision.in_tree(), Element::ONE);
///
/// // the value that couldn't be inserted
/// assert_eq!(collision.inserted(), colliding_element);
/// ```
#[derive(Debug, Clone)]
pub struct CollisionError {
    pub(crate) collisions: Vec<Collision>,
}

impl core::fmt::Display for CollisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CollisionError(length: {})", self.collisions.len())
    }
}

impl std::error::Error for CollisionError {}

impl CollisionError {
    /// A list of all the individual [`Collision`]s that make up this [`CollisionError`]
    #[inline]
    #[must_use]
    pub fn collisions(&self) -> &[Collision] {
        &self.collisions
    }

    pub(crate) fn new() -> Self {
        Self { collisions: vec![] }
    }

    pub(crate) fn push(&mut self, collison: Collision) {
        self.collisions.push(collison);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.collisions.is_empty()
    }
}

/// A single collision in a [`CollisionError`]
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Collision {
    pub(crate) in_tree: Element,
    pub(crate) inserted: Element,
    pub(crate) depth: usize,
    /// The name of the struct that we are inserting into
    pub(crate) struct_name: StructName,
}

/// We want to generate a different message depending on whether this collision was caused by
/// inserting into a batch or a tree
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StructName {
    Batch,
    Tree,
}

impl core::fmt::Display for Collision {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let Self {
            in_tree,
            inserted,
            depth,
            struct_name,
        } = self;

        let struct_name = match struct_name {
            StructName::Batch => "batch",
            StructName::Tree => "tree",
        };

        write!(
            f,
            "collision: tried to insert {inserted}, but {in_tree} was already in the {struct_name}, which have the same least significant {} bits",
            depth - 1
        )
    }
}

impl Collision {
    /// The [`Element`] that was already in the tree
    ///
    /// ```rust
    /// use smirk::*;
    /// use element::Element;
    /// let mut tree: Tree<64, _> = smirk! { 1 };
    /// let collides = Element::ONE + (Element::ONE << 100);
    /// let error = tree.insert(collides, ()).unwrap_err();
    /// let collision = &error.collisions()[0];
    ///
    /// assert_eq!(collision.in_tree(), Element::ONE);
    /// ```
    #[inline]
    #[must_use]
    pub fn in_tree(&self) -> Element {
        self.in_tree
    }

    /// The [`Element`] that was attempted to be inserted
    ///
    /// ```rust
    /// use smirk::*;
    /// use element::Element;
    /// let mut tree: Tree<64, _> = smirk! { 1 };
    /// let collides = Element::ONE + (Element::ONE << 100);
    /// let error = tree.insert(collides, ()).unwrap_err();
    /// let collision = &error.collisions()[0];
    ///
    /// assert_eq!(collision.inserted(), collides);
    /// ```
    #[inline]
    #[must_use]
    pub fn inserted(&self) -> Element {
        self.inserted
    }
}
