use MiniBufferEvent::{LoadMap, SaveMap, UnfocusMB};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{buffer::Buffer, layout::Rect, style::Color, text::Line, widgets::Widget};

pub enum MiniBufferEvent {
    UnfocusMB,
    SaveMap(String),
    LoadMap(String),
}

pub struct MiniBuffer {
    pub text: String,
    color: Color,
}

impl MiniBuffer {
    pub const fn new() -> Self {
        Self {
            text: String::new(),
            color: Color::White,
        }
    }

    pub fn on_enter(&mut self) {
        self.color = Color::White;
        self.text.clear();
        self.set_text(":".to_string(), Color::White);
    }

    pub fn set_text(&mut self, text: String, color: Color) {
        self.text = text;
        self.color = color;
    }

    fn process_command(cmd: &str) -> Option<MiniBufferEvent> {
        match cmd.split(' ').collect::<Vec<&str>>()[..] {
            ["w", filename] => Some(SaveMap(filename.to_string())),
            ["e", filename] => Some(LoadMap(filename.to_string())),
            [] | [_] | [_, _, _, ..] | [&_, _] => None,
        }
    }

    pub fn handle_events(&mut self, key_event: KeyEvent) -> Option<MiniBufferEvent> {
        match key_event.code {
            KeyCode::Esc => {
                self.text.clear();
                Some(UnfocusMB)
            }
            KeyCode::Backspace => {
                let _ = self.text.pop();
                None
            }
            KeyCode::Char(c) => {
                self.text.push(c);
                None
            }
            KeyCode::Enter => Self::process_command(&self.text),
            _ => None,
        }
    }
}

impl Widget for &MiniBuffer {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let message = Line::from(self.text.clone()).style(self.color);
        message.render(area, buf);
    }
}
