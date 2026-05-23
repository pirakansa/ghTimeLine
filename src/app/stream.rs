use eframe::egui;

use crate::models::{
    AppConfig, LibraryView, SavedQuery, Selection, SortOrder, StreamFilter, StreamItem,
};

pub struct StreamState {
    pub selection: Selection,
    pub filter: Option<StreamFilter>,
    new_query_name: String,
    new_query_text: String,
    polling_interval_draft: u16,
}

impl Default for StreamState {
    fn default() -> Self {
        Self {
            selection: Selection::Library(LibraryView::Inbox),
            filter: None,
            new_query_name: String::new(),
            new_query_text: String::new(),
            polling_interval_draft: 5,
        }
    }
}

pub enum StreamEvent {
    Select(Selection),
    SetFilter(Option<StreamFilter>),
    AddQuery { name: String, query: String },
    DeleteSelectedQuery,
    RefreshNow,
    SetDefaultSort(SortOrder),
    SetPollingInterval(u16),
    ItemAction(ItemAction),
}

pub enum ItemAction {
    MarkRead(i64),
    MarkUnread(i64),
    Bookmark(i64, bool),
    Archive(i64),
    Open(String),
}

pub fn show(
    ctx: &egui::Context,
    state: &mut StreamState,
    config: &AppConfig,
    saved_queries: &[SavedQuery],
    items: &[StreamItem],
    status: &str,
) -> Option<StreamEvent> {
    state.polling_interval_draft = config.refresh.polling_interval_minutes;
    let mut event = None;

    egui::SidePanel::left("stream-left")
        .resizable(true)
        .default_width(260.0)
        .show(ctx, |ui| {
            ui.heading("Library");
            for library in LibraryView::ALL {
                if ui
                    .selectable_label(
                        state.selection == Selection::Library(library),
                        library.label(),
                    )
                    .clicked()
                {
                    event = Some(StreamEvent::Select(Selection::Library(library)));
                }
            }

            ui.separator();
            ui.heading("Saved queries");
            for query in saved_queries {
                let label = format!("{} ({})", query.name, query.unread_count);
                if ui
                    .selectable_label(state.selection == Selection::SavedQuery(query.id), label)
                    .clicked()
                {
                    event = Some(StreamEvent::Select(Selection::SavedQuery(query.id)));
                }
            }

            ui.separator();
            ui.label("New query");
            ui.text_edit_singleline(&mut state.new_query_name);
            ui.text_edit_singleline(&mut state.new_query_text);
            if ui.button("Add").clicked() {
                event = Some(StreamEvent::AddQuery {
                    name: state.new_query_name.clone(),
                    query: state.new_query_text.clone(),
                });
                state.new_query_name.clear();
                state.new_query_text.clear();
            }

            if matches!(state.selection, Selection::SavedQuery(_))
                && ui.button("Delete selected").clicked()
            {
                event = Some(StreamEvent::DeleteSelectedQuery);
            }
        });

    egui::TopBottomPanel::bottom("stream-status").show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(status);
            ui.separator();
            ui.label(format!("Host: {}", config.host.name));
            ui.separator();
            ui.label(format!(
                "PAT: {}",
                crate::config::redact_pat(&config.auth.pat)
            ));
        });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Refresh").clicked() {
                event = Some(StreamEvent::RefreshNow);
            }

            ui.separator();
            ui.label("Filter");
            if ui.selectable_label(state.filter.is_none(), "All").clicked() {
                event = Some(StreamEvent::SetFilter(None));
            }
            for filter in StreamFilter::ALL {
                if ui
                    .selectable_label(state.filter == Some(filter), filter.label())
                    .clicked()
                {
                    event = Some(StreamEvent::SetFilter(Some(filter)));
                }
            }

            ui.separator();
            egui::ComboBox::from_id_salt("default-sort")
                .selected_text(config.ui.default_sort.label())
                .show_ui(ui, |ui| {
                    for sort in SortOrder::ALL {
                        if ui
                            .selectable_label(config.ui.default_sort == sort, sort.label())
                            .clicked()
                        {
                            event = Some(StreamEvent::SetDefaultSort(sort));
                        }
                    }
                });

            ui.separator();
            ui.add(
                egui::DragValue::new(&mut state.polling_interval_draft)
                    .range(1..=1440)
                    .speed(1),
            );
            ui.label("min");
            if state.polling_interval_draft != config.refresh.polling_interval_minutes
                && ui.button("Save interval").clicked()
            {
                event = Some(StreamEvent::SetPollingInterval(
                    state.polling_interval_draft,
                ));
            }
        });

        ui.separator();

        if items.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("0 items");
            });
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for item in items {
                    draw_item(ui, item, &mut event);
                    ui.separator();
                }
            });
        }
    });

    event
}

fn draw_item(ui: &mut egui::Ui, item: &StreamItem, event: &mut Option<StreamEvent>) {
    ui.horizontal(|ui| {
        let unread_marker = if item.is_unread { "Unread" } else { "Read" };
        ui.label(unread_marker);
        ui.label(item.item_type.label());
        ui.label(format!("#{}", item.number));
        ui.label(item.state.as_str());
        ui.label(item.updated_at_github.as_str());
    });
    ui.heading(&item.title);
    ui.horizontal_wrapped(|ui| {
        ui.label(item.repository_full_name());
        if let Some(author) = &item.author_login {
            ui.label(format!("by {author}"));
        }
        ui.label(format!("{} comments", item.comment_count));
        if let Some(review_status) = &item.review_status {
            ui.label(format!("review: {review_status}"));
        }
    });
    if !item.assignees.is_empty() {
        ui.label(format!("Assignees: {}", item.assignees.join(", ")));
    }
    if !item.labels.is_empty() {
        ui.label(format!("Labels: {}", item.labels.join(", ")));
    }
    ui.horizontal(|ui| {
        if item.is_unread {
            if ui.button("Mark read").clicked() {
                *event = Some(StreamEvent::ItemAction(ItemAction::MarkRead(item.id)));
            }
        } else if ui.button("Mark unread").clicked() {
            *event = Some(StreamEvent::ItemAction(ItemAction::MarkUnread(item.id)));
        }

        let bookmark_label = if item.is_bookmarked {
            "Remove bookmark"
        } else {
            "Bookmark"
        };
        if ui.button(bookmark_label).clicked() {
            *event = Some(StreamEvent::ItemAction(ItemAction::Bookmark(
                item.id,
                !item.is_bookmarked,
            )));
        }

        if !item.is_archived && ui.button("Archive").clicked() {
            *event = Some(StreamEvent::ItemAction(ItemAction::Archive(item.id)));
        }

        if ui.button("Open").clicked() {
            *event = Some(StreamEvent::ItemAction(ItemAction::Open(
                item.html_url.clone(),
            )));
        }
    });
}
