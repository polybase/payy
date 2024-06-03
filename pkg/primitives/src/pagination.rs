use std::ops::Bound;

use base64::Engine;

/// A wrapper around a value that serializes it using serde_json and then encodes it using base64
#[derive(Debug, Clone, Copy)]
pub struct Opaque<T>(pub T);

impl<T> serde::Serialize for Opaque<T>
where
    T: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = serde_json::to_vec(&self.0).map_err(serde::ser::Error::custom)?;

        let b64 = base64::prelude::BASE64_STANDARD.encode(bytes);

        b64.serialize(serializer)
    }
}

impl<'de, T> serde::Deserialize<'de> for Opaque<T>
where
    T: serde::de::DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Opaque<T>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let b64 = String::deserialize(deserializer)?;

        let bytes = base64::prelude::BASE64_STANDARD
            .decode(b64.as_bytes())
            .map_err(serde::de::Error::custom)?;

        let value = serde_json::de::from_slice(&bytes).map_err(serde::de::Error::custom)?;

        Ok(Opaque(value))
    }
}

impl<T> Opaque<T> {
    pub fn into_inner(self) -> T {
        self.0
    }

    pub fn inner(&self) -> &T {
        &self.0
    }
}

impl<T> std::ops::Deref for Opaque<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Opaque<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> AsRef<T> for Opaque<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Cursor<Pos> {
    pub after: Option<CursorChoiceAfter<Pos>>,
    pub before: Option<CursorChoiceBefore<Pos>>,
}

impl<Pos> Cursor<Pos> {
    pub fn into_opaque(self) -> OpaqueCursor<Pos> {
        OpaqueCursor {
            after: self.after.map(Opaque),
            before: self.before.map(Opaque),
        }
    }
}

/// A variant of [Cursor] that uses a binary encoding for `after` and `before`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(bound = "Pos: serde::Serialize + serde::de::DeserializeOwned")]
pub struct OpaqueCursor<Pos> {
    pub after: Option<Opaque<CursorChoiceAfter<Pos>>>,
    pub before: Option<Opaque<CursorChoiceBefore<Pos>>>,
}

/// This type is meant to be used in client code,
/// where the actual cursor types are not needed.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct OpaqueClientCursor {
    pub after: Option<String>,
    pub before: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum CursorChoice<Pos> {
    After(CursorChoiceAfter<Pos>),
    Before(CursorChoiceBefore<Pos>),
}

impl<Pos> CursorChoice<Pos> {
    pub fn opaque(self) -> OpaqueCursorChoice<Pos> {
        Opaque(self)
    }
}

pub type OpaqueCursorChoice<Pos> = Opaque<CursorChoice<Pos>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CursorChoiceAfter<Pos> {
    After(Pos),
    AfterInclusive(Pos),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CursorChoiceBefore<Pos> {
    Before(Pos),
    BeforeInclusive(Pos),
}

impl<Pos> CursorChoiceAfter<Pos> {
    pub fn inner(&self) -> &Pos {
        match self {
            Self::After(pos) => pos,
            Self::AfterInclusive(pos) => pos,
        }
    }

    pub fn inclusive(self) -> CursorChoiceAfter<Pos> {
        match self {
            Self::After(pos) => CursorChoiceAfter::AfterInclusive(pos),
            Self::AfterInclusive(pos) => CursorChoiceAfter::AfterInclusive(pos),
        }
    }

    pub fn to_bound(&self) -> Bound<&Pos> {
        match self {
            Self::After(pos) => Bound::Excluded(pos),
            Self::AfterInclusive(pos) => Bound::Included(pos),
        }
    }

    pub fn map_pos<F, NewPos>(&self, f: F) -> CursorChoiceAfter<NewPos>
    where
        F: FnOnce(&Pos) -> NewPos,
    {
        match self {
            Self::After(pos) => CursorChoiceAfter::After(f(pos)),
            Self::AfterInclusive(pos) => CursorChoiceAfter::AfterInclusive(f(pos)),
        }
    }
}

impl<Pos> CursorChoiceBefore<Pos> {
    pub fn inner(&self) -> &Pos {
        match self {
            Self::Before(pos) => pos,
            Self::BeforeInclusive(pos) => pos,
        }
    }

    pub fn inclusive(self) -> CursorChoiceBefore<Pos> {
        match self {
            Self::Before(pos) => CursorChoiceBefore::BeforeInclusive(pos),
            Self::BeforeInclusive(pos) => CursorChoiceBefore::BeforeInclusive(pos),
        }
    }

    pub fn to_bound(&self) -> Bound<&Pos> {
        match self {
            Self::Before(pos) => Bound::Excluded(pos),
            Self::BeforeInclusive(pos) => Bound::Included(pos),
        }
    }

    pub fn map_pos<F, NewPos>(&self, f: F) -> CursorChoiceBefore<NewPos>
    where
        F: FnOnce(&Pos) -> NewPos,
    {
        match self {
            Self::Before(pos) => CursorChoiceBefore::Before(f(pos)),
            Self::BeforeInclusive(pos) => CursorChoiceBefore::BeforeInclusive(f(pos)),
        }
    }
}

impl<Pos> CursorChoice<Pos> {
    pub fn inner(&self) -> &Pos {
        match self {
            Self::After(after) => after.inner(),
            Self::Before(before) => before.inner(),
        }
    }

    pub fn map_pos<F, NewPos>(&self, f: F) -> CursorChoice<NewPos>
    where
        F: FnOnce(&Pos) -> NewPos,
    {
        match self {
            Self::After(after) => CursorChoice::After(after.map_pos(f)),
            Self::Before(before) => CursorChoice::Before(before.map_pos(f)),
        }
    }

    pub fn inclusive(self) -> CursorChoice<Pos> {
        match self {
            Self::After(after) => Self::After(after.inclusive()),
            Self::Before(before) => Self::Before(before.inclusive()),
        }
    }
}

pub struct Paginator<I: Iterator, Pos, F> {
    iter: I,
    item_to_pos: F,
    before: Option<CursorChoiceBefore<Pos>>,
    after: Option<CursorChoiceAfter<Pos>>,
}

impl<I, Pos, F> Paginator<I, Pos, F>
where
    I: Iterator,
    F: FnMut(&I::Item) -> Option<Pos>,
    Pos: Clone,
{
    pub fn new(iter: I, item_to_pos: F) -> Self {
        Self {
            iter,
            item_to_pos,
            before: None,
            after: None,
        }
    }

    pub fn collect<B: FromIterator<I::Item>>(mut self) -> (Cursor<Pos>, B) {
        let collected = <&mut Paginator<I, Pos, F> as Iterator>::collect(&mut self);

        (
            Cursor {
                before: self.before,
                after: self.after,
            },
            collected,
        )
    }
}

impl<I, Pos, F> Iterator for Paginator<I, Pos, F>
where
    I: Iterator,
    F: FnMut(&I::Item) -> Option<Pos>,
    Pos: Clone,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iter.next()?;

        let pos = (self.item_to_pos)(&item);

        if let Some(pos) = pos {
            if self.before.is_none() {
                self.before = Some(CursorChoiceBefore::Before(pos.clone()));
            }

            self.after = Some(CursorChoiceAfter::After(pos));
        }

        Some(item)
    }
}

pub struct PaginatedList<L, Pos> {
    pub cursor: Cursor<Pos>,
    pub list: L,
}

impl<L, Pos, E> From<PaginatedList<Result<L, E>, Pos>> for Result<PaginatedList<L, Pos>, E> {
    fn from(paginated_list: PaginatedList<Result<L, E>, Pos>) -> Self {
        let list = paginated_list.list?;
        Ok(PaginatedList {
            cursor: paginated_list.cursor,
            list,
        })
    }
}
