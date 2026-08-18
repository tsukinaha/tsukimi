#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentAction {
    PublishCurrent(u64),
    Clear(u64),
    None,
}

pub fn content_action(
    previous_selected: Option<u64>, selected: Option<u64>, selected_content_changed: bool,
) -> ContentAction {
    if previous_selected != selected {
        return selected
            .map(ContentAction::PublishCurrent)
            .or_else(|| previous_selected.map(ContentAction::Clear))
            .unwrap_or(ContentAction::None);
    }
    if selected_content_changed {
        selected
            .map(ContentAction::PublishCurrent)
            .unwrap_or(ContentAction::None)
    } else {
        ContentAction::None
    }
}
