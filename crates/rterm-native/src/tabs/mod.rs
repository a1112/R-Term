use rterm_domain::TabId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabPresentation {
    pub tab: TabId,
    pub title: String,
    pub active: bool,
}

impl TabPresentation {
    #[must_use]
    pub fn new(tab: TabId, title: impl Into<String>, active: bool) -> Self {
        Self {
            tab,
            title: title.into(),
            active,
        }
    }
}
