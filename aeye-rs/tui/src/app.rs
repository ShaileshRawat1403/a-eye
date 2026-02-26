use anyhow::Result;
use crossterm::event;
use crossterm::terminal;
use ratatui::prelude::*;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;

use crate::args::Args;

pub struct App {
    args: Args,
    should_quit: bool,
    terminal: Terminal<CrosstermBackend<std::io::Stderr>>,
}

impl App {
    pub fn new(args: Args) -> Result<Self> {
        let terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;

        Ok(Self {
            args,
            should_quit: false,
            terminal,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;

        loop {
            if self.should_quit {
                break;
            }
            self.draw()?;
            self.handle_input()?;
        }

        terminal::disable_raw_mode()?;
        Ok(())
    }

    fn draw(&mut self) -> Result<()> {
        self.terminal.draw(|f| {
            let area = f.area();
            let paragraph = Paragraph::new("A-Eye CLI\n\nUse :quit to exit")
                .block(Block::bordered().title("A-Eye"))
                .centered();
            f.render_widget(paragraph, area);
        })?;
        Ok(())
    }

    fn handle_input(&mut self) -> Result<()> {
        if let event::Event::Key(key) = event::read()? {
            if key.code == event::KeyCode::Char('q') {
                self.should_quit = true;
            }
        }
        Ok(())
    }
}
