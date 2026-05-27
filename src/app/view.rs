use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use chrono::{DateTime, FixedOffset};

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
        let local_filter = self.stream.local_filter.clone();
        if let AppMode::Main(runtime) = &mut self.mode {
            let sort = current_sort(runtime);
            match load_current_view(
                &runtime.storage,
                runtime.host_id,
                &selection,
                filter,
                local_filter.as_deref(),
                sort,
            ) {
                Ok(items) => {
                    runtime.items = items;
                    self.stream.pending_remote_item_ids.clear();
                }
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not load stream items: {err}"),
                ),
            }
        }
    }

    pub(super) fn defer_current_view_updates(&mut self, changed_item_ids: &[i64]) {
        if changed_item_ids.is_empty() && self.stream.pending_remote_item_ids.is_empty() {
            return;
        }

        let selection = self.stream.selection.clone();
        let filter = self.stream.filter;
        let local_filter = self.stream.local_filter.clone();
        if let AppMode::Main(runtime) = &mut self.mode {
            let sort = current_sort(runtime);
            match load_current_view(
                &runtime.storage,
                runtime.host_id,
                &selection,
                filter,
                local_filter.as_deref(),
                sort,
            ) {
                Ok(latest_items) if latest_items == runtime.items => {
                    self.stream.pending_remote_item_ids.clear();
                }
                Ok(latest_items) => {
                    let candidate_ids = self
                        .stream
                        .pending_remote_item_ids
                        .iter()
                        .copied()
                        .chain(changed_item_ids.iter().copied())
                        .collect::<HashSet<_>>();
                    self.stream.pending_remote_item_ids = candidate_ids
                        .into_iter()
                        .filter(|item_id| {
                            runtime.items.iter().find(|item| item.id == *item_id)
                                != latest_items.iter().find(|item| item.id == *item_id)
                        })
                        .collect();
                }
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
        let local_filter = self.stream.local_filter.clone();
        if let AppMode::Main(runtime) = &mut self.mode {
            let sort = current_sort(runtime);
            match runtime.storage.list_items_for_selection_by_ids(
                runtime.host_id,
                &selection,
                filter,
                local_filter.as_deref(),
                sort,
                changed_item_ids,
            ) {
                Ok(changed_items) => {
                    if !self.stream.pending_remote_item_ids.is_empty() {
                        patch_local_item_state(
                            &mut runtime.items,
                            changed_item_ids,
                            &changed_items,
                        );
                    } else if current_view_membership_changed(
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
                            local_filter.as_deref(),
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

        if !self.stream.pending_remote_item_ids.is_empty() {
            self.defer_current_view_updates(&[]);
        }
    }

    pub(super) fn set_filter(&mut self, filter: Option<StreamFilter>) {
        self.stream.filter = filter;
        self.reload_current_view();
    }

    pub(super) fn set_local_filter(&mut self, local_filter: Option<String>) {
        let normalized = local_filter
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());

        if let AppMode::Main(runtime) = &mut self.mode {
            if let Err(err) = runtime.storage.validate_local_filter(normalized.as_deref()) {
                Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not apply local filter: {err}"),
                );
                return;
            }
        }

        self.stream.local_filter = normalized.clone();
        self.stream.local_filter_input = normalized.unwrap_or_default();
        self.reload_current_view();
    }

    pub(super) fn add_local_filter_input_term(&mut self, term: &str) {
        let input = self.stream.local_filter_input.trim();
        if input.split_whitespace().any(|existing| existing == term) {
            return;
        }

        self.stream.local_filter_input = if input.is_empty() {
            term.to_owned()
        } else {
            format!("{input} {term}")
        };
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
    local_filter: Option<&str>,
    sort: SortOrder,
) -> crate::storage::Result<Vec<StreamItem>> {
    match selection {
        Selection::Library(library) => {
            storage.list_items_for_library(host_id, *library, filter, local_filter, sort)
        }
        Selection::SavedQuery(id) => {
            storage.list_items_for_saved_query(*id, filter, local_filter, sort)
        }
        Selection::FilterStream(id) => {
            storage.list_items_for_filter_stream(*id, filter, local_filter, sort)
        }
    }
}

fn current_sort(runtime: &Runtime) -> SortOrder {
    runtime.config.ui.default_sort
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
                | SortOrder::ReadDesc
                | SortOrder::ClosedDesc
                | SortOrder::MergedDesc
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

    items.sort_unstable_by(|left, right| compare_items(left, right, sort));
    if items.len() > STREAM_VIEW_LIMIT {
        items.truncate(STREAM_VIEW_LIMIT);
    }
}

fn patch_local_item_state(
    items: &mut Vec<StreamItem>,
    changed_item_ids: &[i64],
    changed_items: &[StreamItem],
) {
    let changed_by_id = changed_items
        .iter()
        .map(|item| (item.id, item))
        .collect::<HashMap<_, _>>();
    let changed_ids = changed_item_ids.iter().copied().collect::<HashSet<_>>();

    items.retain_mut(|item| {
        if !changed_ids.contains(&item.id) {
            return true;
        }
        let Some(changed) = changed_by_id.get(&item.id) else {
            return false;
        };
        item.is_unread = changed.is_unread;
        item.is_bookmarked = changed.is_bookmarked;
        item.is_archived = changed.is_archived;
        true
    });
}

fn compare_items(left: &StreamItem, right: &StreamItem, sort: SortOrder) -> Ordering {
    match sort {
        SortOrder::UpdatedDesc => compare_timestamp_desc(
            &left.updated_at_github,
            &right.updated_at_github,
            left.id,
            right.id,
        ),
        SortOrder::CreatedDesc => compare_timestamp_desc(
            &left.created_at_github,
            &right.created_at_github,
            left.id,
            right.id,
        ),
        SortOrder::ReadDesc => compare_optional_timestamp_desc(
            left.read_at.as_deref(),
            right.read_at.as_deref(),
            left,
            right,
        ),
        SortOrder::ClosedDesc => compare_optional_timestamp_desc(
            left.closed_at_github.as_deref(),
            right.closed_at_github.as_deref(),
            left,
            right,
        ),
        SortOrder::MergedDesc => compare_optional_timestamp_desc(
            left.merged_at_github.as_deref(),
            right.merged_at_github.as_deref(),
            left,
            right,
        ),
    }
}

fn compare_optional_timestamp_desc(
    left: Option<&str>,
    right: Option<&str>,
    left_item: &StreamItem,
    right_item: &StreamItem,
) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => {
            compare_timestamp_desc(left, right, left_item.id, right_item.id)
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => compare_timestamp_desc(
            &left_item.updated_at_github,
            &right_item.updated_at_github,
            left_item.id,
            right_item.id,
        ),
    }
}

fn compare_timestamp_desc(left: &str, right: &str, left_id: i64, right_id: i64) -> Ordering {
    parse_timestamp(right)
        .cmp(&parse_timestamp(left))
        .then_with(|| right.cmp(left))
        .then_with(|| right_id.cmp(&left_id))
}

fn parse_timestamp(value: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(value).ok()
}
