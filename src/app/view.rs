use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::app::{AppMode, GhStreamApp, Runtime};
use crate::models::{LibraryCounts, SavedQuery, Selection, SortOrder, StreamFilter, StreamItem};
use crate::storage::Storage;

const STREAM_VIEW_LIMIT: usize = 500;

impl GhStreamApp {
    pub(super) fn reload_queries(&mut self) {
        if let AppMode::Main(runtime) = &mut self.mode {
            match load_sidebar_data(&runtime.storage, runtime.host_id) {
                Ok((library_counts, saved_queries)) => {
                    runtime.library_counts = library_counts;
                    runtime.saved_queries = saved_queries;
                }
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not load saved queries: {err}"),
                ),
            }
        }
    }

    pub(super) fn reload_current_view(&mut self) {
        let selection = self.stream.selection.clone();
        let filter = self.stream.filter;
        if let AppMode::Main(runtime) = &mut self.mode {
            let sort = current_sort(runtime, &selection);
            match load_current_view(&runtime.storage, runtime.host_id, &selection, filter, sort) {
                Ok(items) => runtime.items = items,
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not load stream items: {err}"),
                ),
            }
        }
    }

    pub(super) fn reload_current_view_for_changed_items(&mut self, changed_item_ids: &[i64]) {
        if changed_item_ids.is_empty() {
            return;
        }

        let selection = self.stream.selection.clone();
        let filter = self.stream.filter;
        if let AppMode::Main(runtime) = &mut self.mode {
            let sort = current_sort(runtime, &selection);
            match runtime.storage.list_items_for_selection_by_ids(
                runtime.host_id,
                &selection,
                filter,
                sort,
                changed_item_ids,
            ) {
                Ok(changed_items) => {
                    if current_view_membership_changed(
                        &runtime.items,
                        changed_item_ids,
                        &changed_items,
                        sort,
                    ) {
                        match load_current_view(
                            &runtime.storage,
                            runtime.host_id,
                            &selection,
                            filter,
                            sort,
                        ) {
                            Ok(items) => runtime.items = items,
                            Err(err) => Self::replace_status_error(
                                &mut self.status,
                                &mut self.status_history,
                                format!("Could not load stream items: {err}"),
                            ),
                        }
                    } else {
                        patch_current_items(&mut runtime.items, changed_items, sort);
                    }
                }
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not load changed stream items: {err}"),
                ),
            }
        }
    }

    pub(super) fn set_filter(&mut self, filter: Option<StreamFilter>) {
        self.stream.filter = filter;
        self.reload_current_view();
    }

    pub(super) fn select(&mut self, selection: Selection) {
        if self.stream.selection != selection {
            self.stream.reset_item_list_scroll = true;
        }
        self.stream.selection = selection;
        self.reload_current_view();
    }
}

pub(super) fn load_sidebar_data(
    storage: &Storage,
    host_id: i64,
) -> crate::storage::Result<(LibraryCounts, Vec<SavedQuery>)> {
    Ok((
        storage.list_library_counts(host_id)?,
        storage.list_saved_queries(host_id)?,
    ))
}

fn load_current_view(
    storage: &Storage,
    host_id: i64,
    selection: &Selection,
    filter: Option<StreamFilter>,
    sort: SortOrder,
) -> crate::storage::Result<Vec<StreamItem>> {
    match selection {
        Selection::Library(library) => {
            storage.list_items_for_library(host_id, *library, filter, sort)
        }
        Selection::SavedQuery(id) => storage.list_items_for_saved_query(*id, filter, sort),
    }
}

fn current_sort(runtime: &Runtime, selection: &Selection) -> SortOrder {
    match selection {
        Selection::SavedQuery(id) => runtime
            .saved_queries
            .iter()
            .find(|query| query.id == *id)
            .map(|query| query.sort)
            .unwrap_or(runtime.config.ui.default_sort),
        Selection::Library(_) => runtime.config.ui.default_sort,
    }
}

fn current_view_membership_changed(
    current_items: &[StreamItem],
    changed_item_ids: &[i64],
    changed_items: &[StreamItem],
    sort: SortOrder,
) -> bool {
    if current_items.len() >= STREAM_VIEW_LIMIT
        && matches!(
            sort,
            SortOrder::UpdatedDesc
                | SortOrder::UpdatedAsc
                | SortOrder::CommentsDesc
                | SortOrder::CommentsAsc
        )
    {
        return true;
    }

    let current_ids = current_items
        .iter()
        .map(|item| item.id)
        .collect::<HashSet<_>>();
    let changed_visible_ids = changed_items
        .iter()
        .map(|item| item.id)
        .collect::<HashSet<_>>();

    changed_item_ids
        .iter()
        .any(|item_id| current_ids.contains(item_id) != changed_visible_ids.contains(item_id))
}

fn patch_current_items(
    items: &mut Vec<StreamItem>,
    changed_items: Vec<StreamItem>,
    sort: SortOrder,
) {
    let mut changed_by_id = changed_items
        .into_iter()
        .map(|item| (item.id, item))
        .collect::<HashMap<_, _>>();

    for item in items.iter_mut() {
        if let Some(changed) = changed_by_id.remove(&item.id) {
            *item = changed;
        }
    }

    if !matches!(sort, SortOrder::CreatedDesc | SortOrder::CreatedAsc) {
        items.sort_unstable_by(|left, right| compare_items(left, right, sort));
    }
    if items.len() > STREAM_VIEW_LIMIT {
        items.truncate(STREAM_VIEW_LIMIT);
    }
}

fn compare_items(left: &StreamItem, right: &StreamItem, sort: SortOrder) -> Ordering {
    match sort {
        SortOrder::UpdatedDesc => right
            .updated_at_github
            .cmp(&left.updated_at_github)
            .then_with(|| right.id.cmp(&left.id)),
        SortOrder::UpdatedAsc => left
            .updated_at_github
            .cmp(&right.updated_at_github)
            .then_with(|| left.id.cmp(&right.id)),
        SortOrder::CreatedDesc | SortOrder::CreatedAsc => Ordering::Equal,
        SortOrder::CommentsDesc => right
            .comment_count
            .cmp(&left.comment_count)
            .then_with(|| right.updated_at_github.cmp(&left.updated_at_github))
            .then_with(|| right.id.cmp(&left.id)),
        SortOrder::CommentsAsc => left
            .comment_count
            .cmp(&right.comment_count)
            .then_with(|| right.updated_at_github.cmp(&left.updated_at_github))
            .then_with(|| left.id.cmp(&right.id)),
    }
}
