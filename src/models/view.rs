#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamFilter {
    Open,
    Unread,
    Bookmarked,
}

impl StreamFilter {
    pub const ALL: [Self; 3] = [Self::Open, Self::Unread, Self::Bookmarked];

    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Unread => "Unread",
            Self::Bookmarked => "Bookmarked",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryView {
    Inbox,
    Bookmark,
    Archived,
}

impl LibraryView {
    pub const ALL: [Self; 3] = [Self::Inbox, Self::Bookmark, Self::Archived];

    pub fn label(self) -> &'static str {
        match self {
            Self::Inbox => "Inbox",
            Self::Bookmark => "Bookmark",
            Self::Archived => "Archived",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selection {
    Library(LibraryView),
    SavedQuery(i64),
    FilterStream(i64),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LibraryCounts {
    pub inbox_unread_count: i64,
    pub bookmark_unread_count: i64,
    pub archived_unread_count: i64,
}

impl LibraryCounts {
    pub fn unread_count(&self, library: LibraryView) -> i64 {
        match library {
            LibraryView::Inbox => self.inbox_unread_count,
            LibraryView::Bookmark => self.bookmark_unread_count,
            LibraryView::Archived => self.archived_unread_count,
        }
    }
}
