use std::collections::{VecDeque, vec_deque};
use std::ops::{Index, RangeBounds};

/// Oldest-to-newest terminal history addressed by logical row index.
///
/// # Compatibility and migration
///
/// The 0.1.0 API intentionally changes [`crate::Terminal::scrollback`] from
/// `&[ScrollbackLine]` to `&HistoryBuffer<ScrollbackLine>`. This breaking
/// pre-1.0 decision permits bounded history to use `VecDeque` internally:
/// removing the oldest row no longer relocates every surviving row.
///
/// Consumers should migrate as follows:
///
/// - keep using [`len`](Self::len), [`is_empty`](Self::is_empty), logical
///   indexing, or [`get`](Self::get);
/// - replace slice iteration with [`iter`](Self::iter) or
///   `for row in terminal.scrollback()`;
/// - replace slice subranges with [`range`](Self::range);
/// - if an external API truly requires a contiguous owned slice, explicitly
///   collect with `history.iter().cloned().collect::<Vec<_>>()`.
///
/// ```
/// use rterm_types::TerminalSize;
/// use rterm_terminal::{ScrollbackLine, Terminal};
///
/// let mut terminal = Terminal::new(TerminalSize::new(4, 2));
/// terminal.feed(b"aa\r\nbb\r\ncc");
/// let history = terminal.scrollback();
///
/// assert_eq!(history.len(), 1);
/// let first: &ScrollbackLine = history.get(0).unwrap();
/// assert!(std::ptr::eq(first, &history[0]));
/// assert_eq!(history.iter().count(), 1);
/// assert_eq!(history.range(..1).count(), 1);
/// assert_eq!(history.into_iter().count(), 1);
/// ```
///
/// `HistoryBuffer` deliberately exposes no contiguous or physical-slice API.
/// These examples must not compile:
///
/// ```compile_fail,E0308
/// use rterm_types::TerminalSize;
/// use rterm_terminal::{ScrollbackLine, Terminal};
///
/// let terminal = Terminal::new(TerminalSize::new(4, 2));
/// let history = terminal.scrollback();
/// let contiguous: &[ScrollbackLine] = history;
/// # let _ = contiguous;
/// ```
///
/// ```compile_fail,E0599
/// use rterm_types::TerminalSize;
/// use rterm_terminal::Terminal;
///
/// let terminal = Terminal::new(TerminalSize::new(4, 2));
/// let history = terminal.scrollback();
/// let physical_segments = history.as_slices();
/// # let _ = physical_segments;
/// ```
///
/// # Complexity
///
/// Logical `len`, `get`, indexing, and `last` are O(1). Appending is amortized
/// O(1). Evicting one front row is O(1), while evicting `k` front rows is O(k)
/// because all removed values must be dropped. A complete terminal prune is
/// O(k + metadata), where `metadata` is the number of coordinate-bearing
/// metadata records rebased for that batch. Iteration and ranges are linear in
/// the number of yielded rows, and rebuilding `n` rows is O(n). Front eviction
/// performs zero survivor-row relocations; this does not make the complete
/// prune operation O(1).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HistoryBuffer<T> {
    rows: VecDeque<T>,
}

impl<T> HistoryBuffer<T> {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            rows: VecDeque::new(),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.rows.get(index)
    }

    #[must_use]
    pub fn last(&self) -> Option<&T> {
        self.rows.back()
    }

    #[must_use]
    pub fn iter(&self) -> vec_deque::Iter<'_, T> {
        self.rows.iter()
    }

    #[must_use]
    pub(crate) fn iter_mut(&mut self) -> vec_deque::IterMut<'_, T> {
        self.rows.iter_mut()
    }

    #[must_use]
    pub fn range<R>(&self, range: R) -> vec_deque::Iter<'_, T>
    where
        R: RangeBounds<usize>,
    {
        self.rows.range(range)
    }

    pub(crate) fn push(&mut self, row: T) {
        self.rows.push_back(row);
    }

    pub(crate) fn evict_front(&mut self, count: usize) -> usize {
        let count = count.min(self.rows.len());
        if count == self.rows.len() {
            self.clear();
            return count;
        }
        for _ in 0..count {
            let _ = self.rows.pop_front();
        }
        count
    }

    pub(crate) fn clear(&mut self) {
        self.rows.clear();
    }

    pub(crate) fn rebuild(&mut self, rows: impl IntoIterator<Item = T>) {
        self.rows = rows.into_iter().collect();
    }
}

impl<T> Index<usize> for HistoryBuffer<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
            .expect("history index must be within the logical row range")
    }
}

impl<'a, T> IntoIterator for &'a HistoryBuffer<T> {
    type Item = &'a T;
    type IntoIter = vec_deque::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut HistoryBuffer<T> {
    type Item = &'a mut T;
    type IntoIter = vec_deque::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::HistoryBuffer;

    #[test]
    fn logical_access_survives_physical_wraparound() {
        let mut history = HistoryBuffer {
            rows: VecDeque::with_capacity(4),
        };
        for value in 0..4 {
            history.push(value);
        }
        assert_eq!(history.evict_front(2), 2);
        history.push(4);
        history.push(5);

        assert_eq!(history.len(), 4);
        assert_eq!(history.get(0), Some(&2));
        assert_eq!(history[1], 3);
        assert_eq!(history.last(), Some(&5));
        assert_eq!(history.iter().copied().collect::<Vec<_>>(), [2, 3, 4, 5]);
        assert_eq!(
            (&history).into_iter().copied().collect::<Vec<_>>(),
            [2, 3, 4, 5]
        );
        assert_eq!(history.range(1..3).copied().collect::<Vec<_>>(), [3, 4]);
    }

    #[test]
    fn rebuild_and_clear_replace_logical_history() {
        let mut history = HistoryBuffer::new();
        history.rebuild([7, 8, 9]);

        assert_eq!(history.iter().copied().collect::<Vec<_>>(), [7, 8, 9]);

        history.clear();

        assert!(history.is_empty());
    }

    #[test]
    fn mutable_iteration_follows_logical_order() {
        let mut history = HistoryBuffer::new();
        history.rebuild([1, 2, 3]);

        for value in &mut history {
            *value *= 2;
        }

        assert_eq!(history.iter().copied().collect::<Vec<_>>(), [2, 4, 6]);
    }
}
