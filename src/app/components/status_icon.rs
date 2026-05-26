use eframe::egui;

use crate::models::{ItemType, StreamItem};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusIcon {
    IssueOpen,
    IssueClosed,
    PullRequestOpen,
    PullRequestClosed,
    PullRequestDraft,
    PullRequestMerged,
}

impl StatusIcon {
    pub fn for_item(item: &StreamItem) -> Self {
        match item.item_type {
            ItemType::Issue => {
                if item.state.eq_ignore_ascii_case("closed") {
                    Self::IssueClosed
                } else {
                    Self::IssueOpen
                }
            }
            ItemType::PullRequest => {
                if item.is_merged.unwrap_or(false) {
                    Self::PullRequestMerged
                } else if item.is_draft.unwrap_or(false) {
                    Self::PullRequestDraft
                } else if item.state.eq_ignore_ascii_case("closed") {
                    Self::PullRequestClosed
                } else {
                    Self::PullRequestOpen
                }
            }
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::IssueOpen => "Issue open",
            Self::IssueClosed => "Issue closed",
            Self::PullRequestOpen => "Pull request open",
            Self::PullRequestClosed => "Pull request closed",
            Self::PullRequestDraft => "Pull request draft",
            Self::PullRequestMerged => "Pull request merged",
        }
    }

    fn color(self) -> egui::Color32 {
        match self {
            Self::IssueOpen | Self::PullRequestOpen => egui::Color32::from_rgb(31, 136, 61),
            Self::IssueClosed | Self::PullRequestClosed => egui::Color32::from_rgb(207, 34, 46),
            Self::PullRequestDraft => egui::Color32::from_rgb(101, 109, 118),
            Self::PullRequestMerged => egui::Color32::from_rgb(130, 80, 223),
        }
    }
}

pub fn show(ui: &mut egui::Ui, icon: StatusIcon) -> egui::Response {
    let icon_size = size_for_ui(ui);
    let desired_size = egui::vec2(icon_size, icon_size);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        match icon {
            StatusIcon::IssueOpen => draw_issue_open(painter, rect, icon.color()),
            StatusIcon::IssueClosed => draw_issue_closed(painter, rect, icon.color()),
            StatusIcon::PullRequestOpen
            | StatusIcon::PullRequestClosed
            | StatusIcon::PullRequestDraft => draw_pull_request(
                painter,
                rect,
                icon.color(),
                icon == StatusIcon::PullRequestDraft,
            ),
            StatusIcon::PullRequestMerged => draw_pull_request_merged(painter, rect, icon.color()),
        }
    }

    response
}

pub fn size_for_ui(ui: &egui::Ui) -> f32 {
    let font_size = egui::TextStyle::Body.resolve(ui.style()).size;
    size_for_font_size(font_size)
}

pub fn size_for_font_size(font_size: f32) -> f32 {
    (font_size * 1.4).clamp(18.0, 32.0)
}

fn draw_issue_open(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let radius = rect.width().min(rect.height()) * 0.33;
    painter.circle_stroke(
        rect.center(),
        radius,
        egui::Stroke::new(stroke_width(rect, 0.1), color),
    );
    painter.circle_filled(rect.center(), radius * 0.34, color);
}

fn draw_issue_closed(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let radius = rect.width().min(rect.height()) * 0.33;
    painter.circle_filled(rect.center(), radius, color);

    let stroke = egui::Stroke::new(stroke_width(rect, 0.1), egui::Color32::WHITE);
    let start = lerp_point(rect, 0.31, 0.54);
    let middle = lerp_point(rect, 0.45, 0.68);
    let end = lerp_point(rect, 0.72, 0.36);
    painter.line_segment([start, middle], stroke);
    painter.line_segment([middle, end], stroke);
}

fn draw_pull_request(
    painter: &egui::Painter,
    rect: egui::Rect,
    color: egui::Color32,
    outline_only: bool,
) {
    let edge = egui::Stroke::new(stroke_width(rect, 0.094), color);
    let node_radius = rect.width().min(rect.height()) * 0.12;
    let top_left = lerp_point(rect, 0.30, 0.24);
    let bottom_left = lerp_point(rect, 0.30, 0.76);
    let top_right = lerp_point(rect, 0.72, 0.24);

    painter.line_segment([top_left, bottom_left], edge);
    painter.line_segment([top_left, top_right], edge);

    draw_node(painter, top_left, node_radius, color, outline_only);
    draw_node(painter, bottom_left, node_radius, color, outline_only);
    draw_node(painter, top_right, node_radius, color, outline_only);
}

fn draw_pull_request_merged(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let edge = egui::Stroke::new(stroke_width(rect, 0.094), color);
    let node_radius = rect.width().min(rect.height()) * 0.12;
    let top_left = lerp_point(rect, 0.28, 0.24);
    let bottom_left = lerp_point(rect, 0.28, 0.76);
    let right_middle = lerp_point(rect, 0.74, 0.50);

    painter.line_segment([top_left, bottom_left], edge);
    painter.line_segment([top_left, right_middle], edge);
    painter.line_segment([bottom_left, right_middle], edge);

    draw_node(painter, top_left, node_radius, color, false);
    draw_node(painter, bottom_left, node_radius, color, false);
    draw_node(painter, right_middle, node_radius, color, false);
}

fn draw_node(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    color: egui::Color32,
    outline_only: bool,
) {
    if outline_only {
        painter.circle_stroke(
            center,
            radius,
            egui::Stroke::new(radius.max(1.4) * 0.55, color),
        );
    } else {
        painter.circle_filled(center, radius, color);
    }
}

fn stroke_width(rect: egui::Rect, scale: f32) -> f32 {
    (rect.width().min(rect.height()) * scale).max(1.4)
}

fn lerp_point(rect: egui::Rect, x: f32, y: f32) -> egui::Pos2 {
    egui::pos2(
        egui::lerp(rect.left()..=rect.right(), x),
        egui::lerp(rect.top()..=rect.bottom(), y),
    )
}

#[cfg(test)]
mod tests {
    use super::{size_for_font_size, StatusIcon};
    use crate::models::{ItemType, StreamItem};

    #[test]
    fn status_icon_maps_issue_states() {
        let mut item = sample_item(ItemType::Issue);
        assert_eq!(StatusIcon::for_item(&item), StatusIcon::IssueOpen);

        item.state = "closed".to_owned();
        assert_eq!(StatusIcon::for_item(&item), StatusIcon::IssueClosed);
    }

    #[test]
    fn status_icon_maps_pull_request_states() {
        let mut item = sample_item(ItemType::PullRequest);
        assert_eq!(StatusIcon::for_item(&item), StatusIcon::PullRequestOpen);

        item.is_draft = Some(true);
        assert_eq!(StatusIcon::for_item(&item), StatusIcon::PullRequestDraft);

        item.is_draft = Some(false);
        item.state = "closed".to_owned();
        assert_eq!(StatusIcon::for_item(&item), StatusIcon::PullRequestClosed);

        item.is_merged = Some(true);
        assert_eq!(StatusIcon::for_item(&item), StatusIcon::PullRequestMerged);
    }

    #[test]
    fn size_scales_with_font_size() {
        assert!((size_for_font_size(10.0) - 18.0).abs() < f32::EPSILON);
        assert!((size_for_font_size(14.0) - 19.6).abs() < 0.01);
        assert!((size_for_font_size(20.0) - 28.0).abs() < f32::EPSILON);
        assert!((size_for_font_size(40.0) - 32.0).abs() < f32::EPSILON);
    }

    fn sample_item(item_type: ItemType) -> StreamItem {
        StreamItem {
            id: 1,
            repository_owner: "owner".to_owned(),
            repository_name: "repo".to_owned(),
            number: 7,
            item_type,
            title: "Example".to_owned(),
            author_login: Some("octo".to_owned()),
            author_avatar_url: Some("https://avatars.githubusercontent.com/u/1?v=4".to_owned()),
            html_url: "https://github.example.test/owner/repo/issues/7".to_owned(),
            state: "open".to_owned(),
            is_draft: Some(false),
            is_merged: Some(false),
            review_status: None,
            comment_count: 0,
            created_at_github: "2026-05-22T00:00:00Z".to_owned(),
            updated_at_github: "2026-05-23T00:00:00Z".to_owned(),
            closed_at_github: None,
            merged_at_github: None,
            read_at: None,
            labels: Vec::new(),
            assignees: Vec::new(),
            review_requests: Vec::new(),
            reviewers: Vec::new(),
            is_unread: true,
            is_bookmarked: false,
            is_archived: false,
        }
    }
}
