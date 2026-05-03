mod color_serde;
mod map_cell;
mod minibuffer;

use std::io;

use color_eyre::eyre::Result;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    style::Color,
};

use crate::{
    map_cell::{CellMap, Mode},
    minibuffer::{MiniBuffer, MiniBufferEvent},
};

enum Focus {
    Map,
    MiniBuffer,
}

struct App {
    exit: bool,
    cell_map: CellMap,
    minibuffer: MiniBuffer,
    focus: Focus,
}

impl App {
    fn build() -> Self {
        Self {
            exit: false,
            cell_map: CellMap::build(),
            minibuffer: MiniBuffer::new(),
            focus: Focus::Map,
        }
    }
    /// # Errors
    ///
    /// It will return an error if it fails to read the terminal's size
    /// or if it cant draw itself in the buffer
    /// or if can't read events from crossterm
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
            self.cell_map.update();
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints(vec![Constraint::Fill(1), Constraint::Length(1)])
            .split(frame.area());

        self.cell_map.set_visible(layout[0]);

        frame.render_widget(&self.cell_map, layout[0]);
        frame.render_widget(&self.minibuffer, layout[1]);
    }

    const fn handle_minibuffer_events(&mut self, ev: &MiniBufferEvent) {
        match ev {
            MiniBufferEvent::UnfocusMB => {
                self.cell_map.set_mode(map_cell::State::Normal);
                self.focus = Focus::Map;
            }
            MiniBufferEvent::CommandSubmited(_) => todo!(),
        }
    }

    fn handle_focus(&mut self, key_event: KeyEvent) {
        match self.focus {
            Focus::Map => self.cell_map.handle_events(key_event),
            Focus::MiniBuffer => {
                if let Some(ev) = self.minibuffer.handle_events(key_event) {
                    self.handle_minibuffer_events(&ev);
                }
            }
        }
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Char('q') => {
                        self.exit = true;
                        return Ok(());
                    }
                    KeyCode::Char(':') => {
                        self.cell_map
                            .set_mode(map_cell::State::Active(Mode::Prompt));
                        self.minibuffer.on_enter();
                        self.focus = Focus::MiniBuffer;
                    }
                    KeyCode::Char('s') => match self.cell_map.save_map() {
                        Ok(msg) => self.minibuffer.set_text(msg, Color::Green),
                        Err(err) => self.minibuffer.set_text(format!("{err}"), Color::Red),
                    },
                    KeyCode::Char('L') => match map_cell::CellMap::load_map() {
                        Ok(new_map) => self.cell_map = new_map,
                        Err(err) => self.minibuffer.set_text(format!("{err}"), Color::Red),
                    },
                    _ => self.handle_focus(key_event),
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut app = App::build();
    Ok(ratatui::run(|terminal| app.run(terminal))?)
}
