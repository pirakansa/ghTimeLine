use std::collections::HashSet;

use eframe::egui;

use super::badges;
use crate::app::components::author_avatar;
use crate::models::{ItemPerson, ItemReview, StreamItem};

pub(super) fn show_author_and_assignees_row(
    ui: &mut egui::Ui,
    item: &StreamItem,
    avatar_cache: &mut author_avatar::AvatarCache,
    avatar_size: f32,
) {
    ui.horizontal(|ui| {
        if let Some(author) = &item.author_login {
            author_avatar::show(
                ui,
                avatar_cache,
                item.author_avatar_url.as_deref(),
                Some(author.as_str()),
            )
            .on_hover_text(author);
        }
        if !item.assignees.is_empty() {
            ui.label(egui::RichText::new("→").weak());
            for assignee in &item.assignees {
                author_avatar::show_sized(
                    ui,
                    avatar_cache,
                    assignee.avatar_url.as_deref(),
                    Some(assignee.login.as_str()),
                    avatar_size,
                )
                .on_hover_text(&assignee.login);
            }
        }
        badges::show_comment_count(ui, item.comment_count);
    });
}

pub(super) fn show_reviewer_row(
    ui: &mut egui::Ui,
    item: &StreamItem,
    avatar_cache: &mut author_avatar::AvatarCache,
    avatar_size: f32,
) {
    if item.review_requests.is_empty() && item.reviewers.is_empty() {
        return;
    }

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), avatar_size),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            let reviewed_logins = reviewed_logins(&item.reviewers, &item.review_requests);
            for reviewer in item.reviewers.iter().rev() {
                show_review_chip(ui, avatar_cache, reviewer, avatar_size);
            }
            for request in item.review_requests.iter().rev() {
                if !is_reviewed_login(reviewed_logins.as_ref(), request.login.as_str()) {
                    show_person_chip(ui, avatar_cache, request, Some("requested"), avatar_size);
                }
            }
            ui.label(egui::RichText::new("←").weak());
        },
    );
}

fn reviewed_logins<'a>(
    reviewers: &'a [ItemReview],
    review_requests: &[ItemPerson],
) -> Option<HashSet<&'a str>> {
    if reviewers.is_empty() || review_requests.is_empty() {
        return None;
    }

    Some(
        reviewers
            .iter()
            .map(|review| review.login.as_str())
            .collect(),
    )
}

fn is_reviewed_login(reviewed_logins: Option<&HashSet<&str>>, login: &str) -> bool {
    reviewed_logins.is_some_and(|reviewed_logins| reviewed_logins.contains(login))
}

fn show_person_chip(
    ui: &mut egui::Ui,
    avatar_cache: &mut author_avatar::AvatarCache,
    person: &ItemPerson,
    review_state: Option<&str>,
    size: f32,
) {
    let response = author_avatar::show_sized(
        ui,
        avatar_cache,
        person.avatar_url.as_deref(),
        Some(person.login.as_str()),
        size,
    )
    .on_hover_text(match review_state {
        Some(state) => format!("{} ({state})", person.login),
        None => person.login.clone(),
    });
    if let Some(state) = review_state {
        badges::paint_review_badge(ui, response.rect, state);
    }
}

fn show_review_chip(
    ui: &mut egui::Ui,
    avatar_cache: &mut author_avatar::AvatarCache,
    review: &ItemReview,
    size: f32,
) {
    let response = author_avatar::show_sized(
        ui,
        avatar_cache,
        review.avatar_url.as_deref(),
        Some(review.login.as_str()),
        size,
    )
    .on_hover_text(format!("{} ({})", review.login, review.state));
    badges::paint_review_badge(ui, response.rect, &review.state);
}
