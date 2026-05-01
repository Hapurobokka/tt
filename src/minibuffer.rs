use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{buffer::Buffer, layout::Rect, text::Line, widgets::Widget};

pub enum MiniBufferEvent {
    UnfocusMB,
    CommandSubmited(String),
}

pub struct MiniBuffer {
    pub text: String,
    prev_text: String,
}

impl MiniBuffer {
    pub fn new() -> Self {
        Self {
            text: String::from("Hola Diego"),
            prev_text: String::new(),
        }
    }

    pub fn on_enter(&mut self) {
        self.prev_text = self.text.clone();
    }

    pub fn handle_events(&mut self, key_event: KeyEvent) -> Option<MiniBufferEvent> {
        match key_event.code {
            KeyCode::Esc => {
                self.text = self.prev_text.clone();
                Some(MiniBufferEvent::UnfocusMB)
            }
            KeyCode::Backspace => {
                let _ = self.text.pop();
                None
            }
            KeyCode::Char(c) => {
                self.text.push(c);
                None
            }
            _ => None,
        }
    }
}

impl Widget for &MiniBuffer {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let message = Line::from(self.text.clone());
        message.render(area, buf);
    }
}
