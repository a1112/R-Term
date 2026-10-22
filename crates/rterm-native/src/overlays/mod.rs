#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayPresentation {
    Search {
        query: String,
        matches: usize,
    },
    CommandPalette {
        query: String,
        selected: Option<usize>,
    },
}
