use crate::app::{stream, AppMode, GhStreamApp};

impl GhStreamApp {
    pub(super) fn item_action(&mut self, action: stream::ItemAction) {
        if let AppMode::Main(runtime) = &mut self.mode {
            let result = match action {
                stream::ItemAction::MarkRead(id) => runtime.storage.set_read_state(id, false),
                stream::ItemAction::MarkUnread(id) => runtime.storage.set_read_state(id, true),
                stream::ItemAction::Bookmark(id, bookmarked) => {
                    runtime.storage.set_bookmarked(id, bookmarked)
                }
                stream::ItemAction::Archive(id, archived) => {
                    runtime.storage.set_archived(id, archived)
                }
                stream::ItemAction::Open(url) => {
                    return match open::that(url) {
                        Ok(()) => {
                            self.status = "Opened in external browser.".to_owned();
                        }
                        Err(err) => {
                            self.status = format!("Could not open browser: {err}");
                        }
                    };
                }
            };
            match result {
                Ok(()) => self.status = "Item state updated.".to_owned(),
                Err(err) => self.status = format!("Could not update item state: {err}"),
            }
        }
        self.reload_queries();
        self.reload_current_view();
    }
}
