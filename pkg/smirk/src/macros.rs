/// Helper macro to create a [`Tree`]
///
/// ```rust
/// # use smirk::*;
/// # use zk_primitives::*;
/// let tree: Tree<64, _> = smirk! {
///   // the element is converted using Element::from
///   123 => "hello",
///   Element::new(234) => "goodbye",
/// };
///
/// assert!(tree.contains_element(Element::new(123)));
/// assert!(tree.contains_element(Element::new(234)));
/// assert!(!tree.contains_element(Element::new(345)));
///
/// // Alternatively, omit `=> expr` to create a `Tree<N, ()>`
/// let tree: Tree<64, _> = smirk! {
///     1,
///     2,
///     3,
/// };
/// ```
///
/// [`Tree`]: crate::Tree
#[macro_export]
macro_rules! smirk {
    {} => {{ $crate::Tree::new() }};
    { $e:expr $(,)? } => {{
        let mut tree = $crate::Tree::new();
        tree.insert($crate::element!($e), ()).unwrap();
        tree
    }};
    { $e:expr, $($t:tt)* } => {{
        let mut tree = $crate::smirk!{ $($t)* };
        tree.insert($crate::element!($e), ()).unwrap();
        tree
    }};
    { $e:expr => $v:expr $(,)? } => {{
        let mut tree = $crate::Tree::new();
        tree.insert($crate::element!($e), $v).unwrap();
        tree
    }};
    { $e:expr => $v:expr, $($t:tt)* } => {{
        let mut tree = $crate::smirk!{ $($t)* };
        tree.insert($crate::element!($e), $v).unwrap();
        tree
    }};
}

/// Helper macro to create a [`Batch`]
///
/// [`Batch`]: crate::Batch
///
/// ```rust
/// # use smirk::*;
/// # use zk_primitives::*;
/// let batch: Batch<64, _> = batch! {
///   // the element is converted using Element::from
///   123 => "hello",
///   Element::new(234) => "goodbye",
/// };
///
///
/// // Alternatively, omit `=> expr` to create a `Batch<N, ()>`
/// let batch: Batch<64, _> = batch! {
///     1,
///     2,
///     3,
/// };
/// ```
///
/// [`Tree`]: crate::Batch
#[macro_export]
macro_rules! batch {
    {} => {{ $crate::Batch::new() }};
    { $e:expr $(,)? } => {{
        let mut batch = $crate::Batch::new();
        batch.insert($crate::element!($e), ()).unwrap();
        batch
    }};
    { $e:expr, $($t:tt)* } => {{
        let mut batch = $crate::batch!{ $($t)* };
        batch.insert($crate::element!($e), ()).unwrap();
        batch
    }};
    { $e:expr => $v:expr $(,)? } => {{
        let mut batch = $crate::Batch::new();
        batch.insert($crate::element!($e), $v).unwrap();
        batch
    }};
    { $e:expr => $v:expr, $($t:tt)* } => {{
        let mut batch = $crate::batch!{ $($t)* };
        batch.insert($crate::element!($e), $v).unwrap();
        batch
    }};
}

/// Helper macro to create an [`Element`]
///
/// [`Element`]: zk_primitives::Element
#[macro_export]
macro_rules! element {
    ($e:literal) => {{
        zk_primitives::Element::new($e)
    }};
    ($e:expr) => {{
        zk_primitives::Element::from($e)
    }};
}

#[cfg(test)]
mod tests {
    use zk_primitives::Element;

    use crate::{smirk, Batch, Tree};

    type T = Tree<64, i32>;
    type B = Batch<64, i32>;

    #[test]
    fn basic_syntax_test() {
        let _t: T = smirk! {};
        let _t: T = smirk! { 1 => 123 };
        let _t: T = smirk! { 1 => 123, };
        let _t: T = smirk! { 1 => 123, 2 => 234 };
        let _t: T = smirk! { 1 => 123, 2 => 234, };

        let identifier = 123;
        let _t: T = smirk! { Element::new(1) => identifier };
        let _t: T = smirk! { Element::new(1) => identifier, };

        let element = Element::new(1);
        let _t: T = smirk! { element => identifier };
        let _t: T = smirk! { element => identifier, };

        let _t: Tree<64, _> = smirk! { 1 };
        let _t: Tree<64, _> = smirk! { 1, };
        let _t: Tree<64, _> = smirk! { 1, 2 };
        let _t: Tree<64, _> = smirk! { 1, 2, };

        let _t: Batch<64, ()> = batch! {};
        let _t: B = batch! { 1 => 123 };
        let _t: B = batch! { 1 => 123, };
        let _t: B = batch! { 1 => 123, 2 => 234 };
        let _t: B = batch! { 1 => 123, 2 => 234, };

        let identifier = 123;
        let _t: B = batch! { Element::new(1) => identifier };
        let _t: B = batch! { Element::new(1) => identifier, };

        let element = Element::new(1);
        let _t: B = batch! { element => identifier };
        let _t: B = batch! { element => identifier, };

        let _t: Batch<64, ()> = batch! { 1 };
        let _t: Batch<64, ()> = batch! { 1, };
        let _t: Batch<64, ()> = batch! { 1, 2 };
        let _t: Batch<64, ()> = batch! { 1, 2, };
    }
}
