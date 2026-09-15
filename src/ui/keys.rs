//! Keyboard shortcuts.

use egui::{Key, Modifiers};

use crate::app::App;
use crate::model::{Action, Dialog, Page, PickerTab};

pub fn handle(app: &mut App, ctx: &egui::Context) {
    let mut actions = Vec::new();
    let chat_open = app.page == Page::Chats && app.open_chat.is_some();
    let can_compose = chat_open && app.recording.is_none();
    let mut insert_question = false;
    let mut question_already_typed = false;
    let mut open_emoji = false;
    let mut edit_last = false;

    ctx.input_mut(|input| {
        // Handle the more specific W shortcut before Ctrl+W. egui's
        // modifier matching permits extra Alt/Shift modifiers for shortcuts.
        if can_compose && input.consume_key(Modifiers::COMMAND | Modifiers::ALT, Key::W) {
            insert_question = true;
            question_already_typed = input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::Text(text) if text == "?"));
        }
        if can_compose && input.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::E) {
            open_emoji = true;
        }
        if chat_open && input.consume_key(Modifiers::COMMAND, Key::ArrowUp) {
            edit_last = true;
        }

        let mut key = |modifiers: Modifiers, key: Key, action: Action| {
            if input.consume_key(modifiers, key) {
                actions.push(action);
            }
        };
        key(Modifiers::COMMAND, Key::F, Action::FocusSearch);
        key(Modifiers::COMMAND, Key::K, Action::FocusSearch);
        key(Modifiers::COMMAND, Key::B, Action::ToggleSidebar);
        key(Modifiers::COMMAND, Key::Comma, Action::Open(Page::Settings));
        key(Modifiers::COMMAND, Key::Q, Action::Quit);
        if chat_open {
            key(Modifiers::COMMAND, Key::W, Action::CloseChat);
        }
        key(
            Modifiers::COMMAND,
            Key::Slash,
            Action::ShowDialog(Dialog::Shortcuts),
        );
        key(Modifiers::COMMAND, Key::Plus, Action::ZoomBy(0.1));
        key(Modifiers::COMMAND, Key::Equals, Action::ZoomBy(0.1));
        key(Modifiers::COMMAND, Key::Minus, Action::ZoomBy(-0.1));
        key(Modifiers::COMMAND, Key::Num0, Action::ResetZoom);
        key(Modifiers::COMMAND, Key::End, Action::ScrollToBottom);
    });

    if insert_question {
        let composer = egui::Id::new("composer-text");
        ctx.memory_mut(|memory| memory.request_focus(composer));
        if !question_already_typed {
            ctx.input_mut(|input| input.events.push(egui::Event::Text("?".to_owned())));
        }
    }
    if open_emoji && app.picker != Some(PickerTab::Emoji) {
        actions.push(Action::TogglePicker(PickerTab::Emoji));
    }
    if edit_last {
        let message = app
            .open_chat
            .as_deref()
            .and_then(|chat| app.conversations.get(chat))
            .and_then(|conversation| {
                conversation
                    .messages
                    .iter()
                    .rev()
                    .find(|message| app.can_edit(message))
            })
            .map(|message| message.id.clone());
        if let Some(message) = message {
            actions.push(Action::Edit(message));
        }
    }

    // Escape cancels the topmost state. Menus handle Escape themselves.
    // With no transient state left, Escape closes the current chat.
    let menu_open = egui::Popup::is_any_open(ctx);
    let search_focused = ctx.memory(|memory| memory.has_focus(egui::Id::new("chat-search")));
    let escape =
        !menu_open && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
    if escape {
        if app.dialog.is_some() {
            actions.push(Action::CloseDialog);
        } else if app.recording.is_some() {
            actions.push(Action::CancelRecording);
        } else if app.picker.is_some() {
            actions.push(Action::ClosePicker);
        } else if app.emoji_start.is_some() {
            actions.push(Action::CloseEmojiSuggestions);
        } else if app.mention_start.is_some() {
            actions.push(Action::CloseMentions);
        } else if !app.pending.is_empty() {
            actions.push(Action::ClearPending);
        } else if app.editing.is_some() {
            actions.push(Action::CancelEdit);
        } else if app.reply_to.is_some() {
            actions.push(Action::CancelReply);
        } else if app.page == Page::Settings {
            actions.push(Action::Open(Page::Chats));
        } else if search_focused || !app.search.is_empty() {
            if !app.search.is_empty() {
                actions.push(Action::Search(String::new()));
            }
            if app.open_chat.is_some() {
                actions.push(Action::FocusComposer);
            }
        } else if app.open_chat.is_some() {
            actions.push(Action::CloseChat);
        }
    }
    // Enter sends a recording because the text field is hidden.
    if app.recording.is_some()
        && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter))
    {
        actions.push(Action::SendRecording);
    }
    // Alt+Up/Down switches chats without leaving the composer.
    let step = ctx.input_mut(|input| {
        if input.consume_key(Modifiers::ALT, Key::ArrowDown) {
            1
        } else if input.consume_key(Modifiers::ALT, Key::ArrowUp) {
            -1
        } else {
            0
        }
    });
    if step != 0 {
        let visible = app.visible_chats();
        if !visible.is_empty() {
            let current = app
                .open_chat
                .as_ref()
                .and_then(|open| visible.iter().position(|chat| chat.id == *open));
            let next = match current {
                Some(index) => (index as i64 + step).rem_euclid(visible.len() as i64) as usize,
                None => 0,
            };
            let next = visible[next].id.clone();
            app.scroll_chat_into_view = Some(next.clone());
            actions.push(Action::OpenChat(next));
        }
    }
    app.actions.extend(actions);
}

/// Shortcuts shown in the help dialog.
pub const SHORTCUTS: &[(&str, &str)] = &[
    ("Ctrl+F / Ctrl+K", "Search chats"),
    ("Alt+↑ / Alt+↓", "Previous / next chat"),
    ("Ctrl+Alt+W", "Insert ? in the composer"),
    ("Ctrl+Shift+E", "Open the emoji picker"),
    ("Ctrl+↑", "Edit the latest editable message you sent"),
    ("Enter", "Send (Shift+Enter for a new line)"),
    (
        "Escape",
        "Dismiss the current action, or close the current chat",
    ),
    ("Ctrl+W", "Close the current chat"),
    ("Ctrl+V", "Paste text, or send a picture from the clipboard"),
    ("Ctrl+B", "Show or hide the chat list"),
    ("Ctrl+End", "Jump to the newest message"),
    ("Ctrl+,", "Settings"),
    ("Ctrl++ / Ctrl+-", "Zoom in / out"),
    ("Ctrl+0", "Reset zoom"),
    ("Ctrl+/", "This list"),
    ("Ctrl+Q", "Quit"),
];

/// Uses Command and Option labels on macOS.
pub fn label(keys: &str) -> String {
    if cfg!(target_os = "macos") {
        keys.replace("Ctrl", "⌘").replace("Alt", "⌥")
    } else {
        keys.to_owned()
    }
}
