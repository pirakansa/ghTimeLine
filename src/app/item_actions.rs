use crate::app::{screens::stream, AppMode, GhStreamApp};

impl GhStreamApp {
    pub(super) fn item_action(&mut self, action: stream::ItemAction) {
        if let AppMode::Main(runtime) = &mut self.mode {
            if let stream::ItemAction::Open { id, url } = action {
                let read_result = runtime.storage.set_read_state(id, false);
                let open_result = open::that(url);

                self.status = match (read_result, open_result) {
                    (Ok(()), Ok(())) => "Opened in external browser.".to_owned(),
                    (Err(err), Ok(())) => {
                        format!("Opened in external browser, but could not mark item read: {err}")
                    }
                    (Ok(()), Err(err)) => format!("Could not open browser: {err}"),
                    (Err(read_err), Err(open_err)) => {
                        format!("Could not open browser: {open_err}; could not mark item read: {read_err}")
                    }
                };

                self.reload_queries();
                self.reload_current_view();
                return;
            }

            let result = match action {
                stream::ItemAction::MarkRead(id) => runtime.storage.set_read_state(id, false),
                stream::ItemAction::MarkUnread(id) => runtime.storage.set_read_state(id, true),
                stream::ItemAction::Bookmark(id, bookmarked) => {
                    runtime.storage.set_bookmarked(id, bookmarked)
                }
                stream::ItemAction::Archive(id, archived) => {
                    runtime.storage.set_archived(id, archived)
                }
                stream::ItemAction::Open { .. } => unreachable!(),
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
