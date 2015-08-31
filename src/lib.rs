extern crate libc;
extern crate nix;

mod error;
mod buffer;
mod history;
mod parser;
mod instr;
mod term;

use std::os::unix::io::{RawFd, AsRawFd};

pub use error::Error;
use history::History;
use buffer::Buffer;
use term::Term;

fn readline_edit(term: &mut Term, history: &History, prompt: &str) -> Result<String, Error> {
    let mut buffer = Buffer::new();
    let mut seq: Vec<u8> = Vec::new();
    let mut history_cursor = history::Cursor::new(history);
    loop {
        try!(term.write(&buffer.get_line(prompt)));
        let byte = try!(try!(term.read_byte()).ok_or(Error::EndOfFile));
        seq.push(byte);

        match parser::parse(&seq) {
            parser::Result::Error => return Err(Error::InvalidUTF8),
            parser::Result::Incomplete => continue,
            parser::Result::Success(token) => {
                match instr::interpret_token(token) {
                    instr::Instr::Done                   => {
                        return Ok(buffer.to_string());
                    },
                    instr::Instr::DeleteCharLeftOfCursor => buffer.delete_char_left_of_cursor(),
                    instr::Instr::MoveCursorLeft         => buffer.move_left(),
                    instr::Instr::MoveCursorRight        => buffer.move_right(),
                    instr::Instr::MoveCursorStart        => buffer.move_start(),
                    instr::Instr::MoveCursorEnd          => buffer.move_end(),
                    instr::Instr::HistoryPrev            => {
                        if history_cursor.incr() {
                            buffer.swap()
                        }
                        history_cursor.get().map(|s| buffer.replace(s));
                    },
                    instr::Instr::HistoryNext            => {
                        if history_cursor.decr() {
                            buffer.swap()
                        }
                        history_cursor.get().map(|s| buffer.replace(s));
                    },
                    instr::Instr::Noop                   => (),
                    instr::Instr::Cancel                 => return Err(Error::EndOfFile),
                    instr::Instr::InsertAtCursor         => try!(buffer.insert_bytes_at_cursor(&seq))
                };
                seq.clear();
            }
        };
    }
}

pub struct Copperline {
    term: Term,
    history: History
}

impl Copperline {

    pub fn new() -> Copperline {
        Copperline::new_from_raw_fds(libc::STDIN_FILENO, libc::STDOUT_FILENO)
    }

    pub fn new_from_io<I: AsRawFd, O: AsRawFd>(i: &I, o: &O) -> Copperline {
        Copperline::new_from_raw_fds(i.as_raw_fd(), o.as_raw_fd())
    }

    pub fn new_from_raw_fds(ifd: RawFd, ofd: RawFd) -> Copperline {
        Copperline {
            term: Term::new(ifd, ofd),
            history: History::new()
        }
    }

    pub fn read_line(&mut self, prompt: &str) -> Result<String, Error> {
        if Term::is_unsupported_term() || !self.term.is_a_tty() {
            return Err(Error::UnsupportedTerm);
        }
        try!(self.term.enable_raw_mode());
        let result = readline_edit(&mut self.term, &self.history, prompt);
        try!(self.term.disable_raw_mode());
        println!("");
        result
    }

    pub fn get_current_history_length(&self) -> usize {
        self.history.len()
    }

    pub fn add_history(&mut self, line: String) {
        self.history.push(line)
    }

    pub fn get_history_item(&self, idx: usize) -> Option<&String> {
        self.history.get(idx)
    }

    pub fn remove_history_item(&mut self, idx: usize) -> Option<String> {
        self.history.remove(idx)
    }

    pub fn clear_history(&mut self) {
        self.history.clear()
    }

}
