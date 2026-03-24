#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![deny(missing_docs)]

//! Selector-guarded note access.

#[cfg(test)]
mod tests;

use element::Element;
use serde::{Deserialize, Serialize};
use zk_primitives::Note;

/// Runtime selector for obtaining notes.
///
/// This currently scopes note retrieval by note kind.
///
/// # Contract
/// The serialized shape is an object so this type can evolve with additional
/// selector fields in the future.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteSelector {
    note_kind: Element,
}

/// A value scoped to a specific note kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteValue {
    /// Note kind for this value.
    pub note_kind: Element,
    /// Value scoped to this note kind.
    pub value: Element,
}

impl NoteSelector {
    /// Creates a note selector for the provided note kind.
    #[must_use]
    pub fn new(note_kind: Element) -> Self {
        Self { note_kind }
    }

    /// Returns the note kind for this selector.
    #[must_use]
    pub fn note_kind(self) -> Element {
        self.note_kind
    }
}

/// A note that has not yet been scoped to a specific selector.
///
/// # Contract
/// - `NoteGuard` intentionally exposes no direct note accessors.
/// - Callers may obtain a `SelectedNote` only through `select`.
/// - `select` enforces selector matching.
/// - Matching is currently defined as `note.note_kind() == selector.note_kind()`.
#[repr(transparent)]
pub struct NoteGuard<N> {
    note: N,
}

/// A note that has already been validated against a selector.
///
/// # Contract
/// `SelectedNote` can only be constructed via `NoteGuard::select`.
#[repr(transparent)]
pub struct SelectedNote<N> {
    note: N,
}

/// Abstraction for types that can expose a note kind discriminator.
pub trait HasNoteKind: std::fmt::Debug {
    /// Returns the note kind discriminator.
    ///
    /// # Contract
    /// Must return the discriminator used by selector matching.
    fn note_kind(&self) -> Element;
}

impl HasNoteKind for Note {
    fn note_kind(&self) -> Element {
        self.contract
    }
}

impl<N> NoteGuard<N> {
    /// Creates a note guard wrapper.
    #[must_use]
    pub fn new(note: N) -> Self {
        Self { note }
    }
}

impl<N> NoteGuard<N>
where
    N: HasNoteKind,
{
    /// Returns `true` when this note satisfies the provided selector.
    ///
    /// # Contract
    /// This is the canonical predicate used by both `get_ref` and `get`.
    #[must_use]
    pub fn matches(&self, selector: NoteSelector) -> bool {
        self.note.note_kind() == selector.note_kind()
    }

    /// Attempts to scope this note for the provided selector.
    ///
    /// # Errors
    ///
    /// Returns `Err(self)` when `matches(selector)` is `false`.
    pub fn select(self, selector: NoteSelector) -> std::result::Result<SelectedNote<N>, Self> {
        if self.matches(selector) {
            return Ok(SelectedNote { note: self.note });
        }

        Err(self)
    }
}

impl<N> SelectedNote<N> {
    /// Returns a reference to the validated note.
    #[must_use]
    pub fn as_note(&self) -> &N {
        &self.note
    }

    /// Returns the validated note.
    #[must_use]
    pub fn into_note(self) -> N {
        self.note
    }
}
