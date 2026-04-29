mod map_cell;

use std::io;

use color_eyre::eyre::Result;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Size},
};

use crate::map_cell::CellMap;

enum Focus {
    Map,
    MiniBuffer,
}

struct App {
    exit: bool,
    cell_map: CellMap,
    focus: Focus,
}

impl App {
    fn build() -> Self {
        Self {
            exit: false,
            cell_map: CellMap::build(),
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

    fn draw(&self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints(vec![Constraint::Fill(1), Constraint::Length(2)])
            .split(frame.area());

        frame.render_widget(&self.cell_map, layout[0]);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                if key_event.code == KeyCode::Char('q') {
                    self.exit = true;
                    return Ok(());
                }
                match self.focus {
                    Focus::Map => self.cell_map.handle_key_press(key_event),
                    Focus::MiniBuffer => todo!(),
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
