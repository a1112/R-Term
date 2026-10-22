use rterm_types::TerminalSize;

use crate::{Cell, SequenceNo};

/// One visible terminal row and all metadata that must travel with it.
#[derive(Debug, Clone)]
pub struct GridRow {
    cells: Vec<Cell>,
    reflow_overflow: Vec<Cell>,
    wrapped: bool,
    last_change_seqno: SequenceNo,
}

impl GridRow {
    fn blank(columns: u16, cell: &Cell, seqno: SequenceNo) -> Self {
        Self {
            cells: vec![cell.clone(); usize::from(columns)],
            reflow_overflow: Vec::new(),
            wrapped: false,
            last_change_seqno: seqno,
        }
    }

    #[must_use]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    #[must_use]
    pub const fn is_wrapped(&self) -> bool {
        self.wrapped
    }

    #[must_use]
    pub const fn last_change_seqno(&self) -> SequenceNo {
        self.last_change_seqno
    }

    #[must_use]
    pub fn reflow_overflow(&self) -> &[Cell] {
        &self.reflow_overflow
    }

    pub(crate) fn into_parts(self) -> (Vec<Cell>, Vec<Cell>, bool, SequenceNo) {
        (
            self.cells,
            self.reflow_overflow,
            self.wrapped,
            self.last_change_seqno,
        )
    }

    fn resize_columns(&mut self, columns: u16, preserve_row_metadata: bool, seqno: SequenceNo) {
        self.cells.resize(usize::from(columns), Cell::default());
        if !preserve_row_metadata {
            self.reflow_overflow.clear();
            self.last_change_seqno = seqno;
        }
    }
}

/// Visible terminal storage addressed by logical row and column.
#[derive(Debug, Clone)]
pub struct TerminalGrid {
    size: TerminalSize,
    rows: Vec<GridRow>,
}

impl TerminalGrid {
    #[must_use]
    pub fn new(size: TerminalSize) -> Self {
        Self::new_with_seqno(size, 1)
    }

    #[must_use]
    pub(crate) fn new_with_seqno(size: TerminalSize, seqno: SequenceNo) -> Self {
        let blank = Cell::default();
        Self {
            size,
            rows: (0..size.rows)
                .map(|_| GridRow::blank(size.columns, &blank, seqno))
                .collect(),
        }
    }

    #[must_use]
    pub const fn size(&self) -> TerminalSize {
        self.size
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.size.cells()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.size.cells() == 0
    }

    #[must_use]
    pub fn get(&self, row: u16, column: u16) -> Option<&Cell> {
        self.rows
            .get(usize::from(row))
            .and_then(|row| row.cells.get(usize::from(column)))
    }

    /// Returns a renderer-neutral view of one viewport row and its row metadata.
    #[must_use]
    pub fn row(&self, row: u16) -> Option<&GridRow> {
        self.rows.get(usize::from(row))
    }

    pub fn set(&mut self, row: u16, column: u16, cell: Cell) -> bool {
        let Some(row) = self.rows.get_mut(usize::from(row)) else {
            return false;
        };
        let Some(slot) = row.cells.get_mut(usize::from(column)) else {
            return false;
        };

        *slot = cell;
        row.reflow_overflow.clear();
        true
    }

    #[must_use]
    pub(crate) fn row_wrapped(&self, row: u16) -> bool {
        self.rows
            .get(usize::from(row))
            .is_some_and(GridRow::is_wrapped)
    }

    pub(crate) fn set_row_wrapped(&mut self, row: u16, wrapped: bool) {
        if let Some(row) = self.rows.get_mut(usize::from(row)) {
            row.wrapped = wrapped;
        }
    }

    #[must_use]
    pub(crate) fn row_last_change_seqno(&self, row: u16) -> Option<SequenceNo> {
        self.rows
            .get(usize::from(row))
            .map(GridRow::last_change_seqno)
    }

    pub(crate) fn set_row_last_change_seqno(&mut self, row: u16, seqno: SequenceNo) -> bool {
        let Some(row) = self.rows.get_mut(usize::from(row)) else {
            return false;
        };
        row.last_change_seqno = seqno;
        true
    }

    #[must_use]
    pub(crate) fn cells_with_reflow_overflow(&self, row: u16) -> Vec<Cell> {
        let Some(row) = self.rows.get(usize::from(row)) else {
            return Vec::new();
        };
        let mut cells = row.cells.clone();
        cells.extend(row.reflow_overflow.iter().cloned());
        cells
    }

    pub(crate) fn set_reflow_overflow(&mut self, row: u16, overflow: Vec<Cell>) {
        if let Some(row) = self.rows.get_mut(usize::from(row)) {
            row.reflow_overflow = overflow;
        }
    }

    pub fn resize(&mut self, size: TerminalSize) {
        let new_row_seqno = self
            .rows
            .iter()
            .map(GridRow::last_change_seqno)
            .max()
            .unwrap_or(1);
        self.resize_with_seqno(size, new_row_seqno);
    }

    pub(crate) fn resize_with_seqno(&mut self, size: TerminalSize, new_row_seqno: SequenceNo) {
        let old_size = self.size;
        let old_rows = std::mem::take(&mut self.rows);
        let preserve_row_metadata = old_size.columns == size.columns;
        let blank = Cell::default();
        let mut old_rows = old_rows.into_iter();

        self.rows = (0..size.rows)
            .map(|_| {
                if let Some(mut row) = old_rows.next() {
                    row.resize_columns(size.columns, preserve_row_metadata, new_row_seqno);
                    row
                } else {
                    GridRow::blank(size.columns, &blank, new_row_seqno)
                }
            })
            .collect();
        self.size = size;
    }

    pub(crate) fn scroll_up_rows(
        &mut self,
        top: u16,
        bottom: u16,
        count: u16,
        blank: &Cell,
        seqno: SequenceNo,
    ) -> Vec<GridRow> {
        let capacity = usize::from(count).min(self.rows.len());
        let mut exiting = Vec::with_capacity(capacity);
        self.rotate_up_rows(top, bottom, count, blank, seqno, |row| exiting.push(row));
        exiting
    }

    pub(crate) fn scroll_up_rows_discarding(
        &mut self,
        top: u16,
        bottom: u16,
        count: u16,
        blank: &Cell,
        seqno: SequenceNo,
    ) {
        self.rotate_up_rows(top, bottom, count, blank, seqno, drop);
    }

    fn rotate_up_rows(
        &mut self,
        top: u16,
        bottom: u16,
        count: u16,
        blank: &Cell,
        seqno: SequenceNo,
        mut consume_exiting: impl FnMut(GridRow),
    ) {
        let columns = self.size.columns;
        let Some(rows) = self.row_region_mut(top, bottom) else {
            return;
        };
        let count = usize::from(count).min(rows.len());
        if count == 0 {
            return;
        }

        rows.rotate_left(count);
        let first_exiting = rows.len() - count;
        for row in &mut rows[first_exiting..] {
            consume_exiting(std::mem::replace(
                row,
                GridRow::blank(columns, blank, seqno),
            ));
        }
    }

    pub(crate) fn scroll_down_rows(
        &mut self,
        top: u16,
        bottom: u16,
        count: u16,
        blank: &Cell,
        seqno: SequenceNo,
    ) {
        let columns = self.size.columns;
        let Some(rows) = self.row_region_mut(top, bottom) else {
            return;
        };
        let count = usize::from(count).min(rows.len());
        if count == 0 {
            return;
        }

        rows.rotate_right(count);
        for row in &mut rows[..count] {
            *row = GridRow::blank(columns, blank, seqno);
        }
    }

    fn row_region_mut(&mut self, top: u16, bottom: u16) -> Option<&mut [GridRow]> {
        if top > bottom || bottom >= self.size.rows {
            return None;
        }
        self.rows.get_mut(usize::from(top)..=usize::from(bottom))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Color;

    fn tagged_grid() -> TerminalGrid {
        let mut grid = TerminalGrid::new_with_seqno(TerminalSize::new(2, 5), 1);
        for row in 0..5 {
            let mut first = Cell::with_char(char::from(b'A' + u8::try_from(row).unwrap()));
            first.foreground = Color::Indexed(u8::try_from(row).unwrap());
            assert!(grid.set(row, 0, first));
            grid.set_reflow_overflow(
                row,
                vec![Cell::with_char(char::from(
                    b'a' + u8::try_from(row).unwrap(),
                ))],
            );
            grid.set_row_wrapped(row, row % 2 == 1);
            assert!(grid.set_row_last_change_seqno(row, 10 + usize::from(row)));
        }
        grid
    }

    fn row_snapshot(grid: &TerminalGrid, row: u16) -> (Vec<Cell>, bool, SequenceNo) {
        (
            grid.cells_with_reflow_overflow(row),
            grid.row_wrapped(row),
            grid.row_last_change_seqno(row).unwrap(),
        )
    }

    #[test]
    fn public_row_view_exposes_cells_wrap_and_change_identity() {
        let grid = tagged_grid();

        let row = grid.row(1).expect("second row");

        assert_eq!(row.cells()[0].primary_char(), 'B');
        assert!(row.is_wrapped());
        assert_eq!(row.last_change_seqno(), 11);
        assert!(grid.row(5).is_none());
    }

    fn styled_blank() -> Cell {
        Cell {
            background: Color::Indexed(9),
            bold: true,
            ..Cell::default()
        }
    }

    #[test]
    fn row_rotation_moves_complete_rows_without_copying_owned_cells() {
        let mut grid = TerminalGrid::new_with_seqno(TerminalSize::new(1, 3), 1);
        let hyperlink = "https://example.test/row-owned-allocation".repeat(4);
        let mut cell = Cell::with_char('x');
        cell.hyperlink = Some(hyperlink.into());
        assert!(grid.set(0, 0, cell));
        grid.set_row_wrapped(0, true);
        grid.set_reflow_overflow(0, vec![Cell::with_char('界')]);
        assert!(grid.set_row_last_change_seqno(0, 99));
        let allocation = grid
            .get(0, 0)
            .and_then(|cell| cell.hyperlink.as_ref())
            .map(|hyperlink| hyperlink.as_ptr())
            .unwrap();

        let exiting = grid.scroll_up_rows(0, 2, 1, &Cell::default(), 100);

        assert_eq!(exiting.len(), 1);
        assert_eq!(exiting[0].cells()[0].primary_char(), 'x');
        assert!(exiting[0].is_wrapped());
        assert_eq!(exiting[0].last_change_seqno(), 99);
        assert_eq!(
            exiting[0].cells()[0].hyperlink.as_ref().unwrap().as_ptr(),
            allocation
        );
        assert_eq!(exiting[0].reflow_overflow[0].primary_char(), '界');
    }

    #[test]
    fn scroll_down_row_rotation_covers_partial_full_overcount_and_noop_regions() {
        let blank = styled_blank();

        let mut partial = tagged_grid();
        let original = (0..5)
            .map(|row| row_snapshot(&partial, row))
            .collect::<Vec<_>>();
        partial.scroll_down_rows(1, 3, 1, &blank, 99);
        assert_eq!(row_snapshot(&partial, 0), original[0]);
        assert_eq!(row_snapshot(&partial, 2), original[1]);
        assert_eq!(row_snapshot(&partial, 3), original[2]);
        assert_eq!(row_snapshot(&partial, 4), original[4]);
        assert_eq!(
            row_snapshot(&partial, 1),
            (vec![blank.clone(), blank.clone()], false, 99)
        );

        for count in [3, 9] {
            let mut full_or_overcount = tagged_grid();
            full_or_overcount.scroll_down_rows(1, 3, count, &blank, 99);
            for row in 1..=3 {
                assert_eq!(
                    row_snapshot(&full_or_overcount, row),
                    (vec![blank.clone(), blank.clone()], false, 99),
                    "count={count}, row={row}"
                );
            }
            assert_eq!(row_snapshot(&full_or_overcount, 0), original[0]);
            assert_eq!(row_snapshot(&full_or_overcount, 4), original[4]);
        }

        for (top, bottom, count) in [(1, 3, 0), (3, 1, 1), (1, 5, 1)] {
            let mut noop = tagged_grid();
            noop.scroll_down_rows(top, bottom, count, &blank, 99);
            assert_eq!(
                (0..5)
                    .map(|row| row_snapshot(&noop, row))
                    .collect::<Vec<_>>(),
                original,
                "top={top}, bottom={bottom}, count={count}"
            );
        }
    }

    #[test]
    fn discard_scroll_up_rotates_rows_without_returning_an_exiting_vec() {
        let mut grid = tagged_grid();
        let blank = styled_blank();
        let original = (0..5)
            .map(|row| row_snapshot(&grid, row))
            .collect::<Vec<_>>();

        let (): () = grid.scroll_up_rows_discarding(1, 3, 1, &blank, 99);

        assert_eq!(row_snapshot(&grid, 0), original[0]);
        assert_eq!(row_snapshot(&grid, 1), original[2]);
        assert_eq!(row_snapshot(&grid, 2), original[3]);
        assert_eq!(row_snapshot(&grid, 4), original[4]);
        assert_eq!(
            row_snapshot(&grid, 3),
            (vec![blank.clone(), blank], false, 99)
        );
    }
}
