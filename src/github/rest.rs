use crate::models::SortOrder;

pub fn search_sort_query(sort: SortOrder) -> (&'static str, &'static str) {
    match sort {
        SortOrder::UpdatedDesc => ("updated", "desc"),
        SortOrder::UpdatedAsc => ("updated", "asc"),
        SortOrder::CreatedDesc => ("created", "desc"),
        SortOrder::CreatedAsc => ("created", "asc"),
        SortOrder::CommentsDesc => ("comments", "desc"),
        SortOrder::CommentsAsc => ("comments", "asc"),
    }
}
