use std::io;

use crossterm::{
    event::{self, Event, KeyCode},
};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Rect, Size},
    symbols::border,
    text::Line,
    widgets::{Block, Widget},
};

#[derive(Debug, Default)]
pub struct App {
    exit: bool,
    player_x: u16,
    player_y: u16,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events(terminal.size()?)?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self, size: Size) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) => match key_event.code {
                KeyCode::Char('q') => self.exit = true,
                KeyCode::Left | KeyCode::Char('h') => {
                    if self.player_x > 1 {
                        self.player_x -= 1
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if self.player_x + 1 <= size.width - 2 {
                        self.player_x += 1
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.player_y > 1 {
                        self.player_y -= 1
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.player_y + 1 <= size.height - 2 {
                        self.player_y += 1
                    }
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for x in 1..area.width - 1 {
            for y in 1..area.height - 1 {
                buf[(area.x + x, area.y + y)].set_char('.');
            }
        }

        buf[(area.x + self.player_x, area.y + self.player_y)].set_char('@');

        let title = Line::from("Prueba");
        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::THICK);
        block.render(area, buf);
    }
}

fn main() -> io::Result<()> {
    let mut app = App {
        exit: false,
        player_x: 1,
        player_y: 1,
    };
    ratatui::run(|terminal| app.run(terminal))
}
