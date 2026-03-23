use crate::Element;

impl Element {
    /// Check to see if this element collides with another element in a tree of depth `DEPTH`
    ///
    /// ```rust
    /// # use element::Element;
    /// const DEPTH: usize = 64;
    ///
    /// let a = Element::new(1);
    /// let b = Element::new(2);
    /// let c = Element::new(1) + (Element::new(1) << 100);
    ///
    /// assert_eq!(a.collides_with::<DEPTH>(a), true);
    /// assert_eq!(a.collides_with::<DEPTH>(b), false);
    /// assert_eq!(a.collides_with::<DEPTH>(c), true);
    /// assert_eq!(a.collides_with::<200>(c), false);
    /// ```
    #[inline]
    #[must_use]
    pub fn collides_with<const DEPTH: usize>(self, other: Element) -> bool {
        self.lsb(DEPTH - 1) == other.lsb(DEPTH - 1)
    }
}
