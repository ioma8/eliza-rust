//! stdin -> reply lines, for cross-validation against the original BASIC.
//! Echoes each input line prefixed with "? " and the reply underneath, so a
//! diff against the PC-BASIC capture is straightforward.

use std::io::{self, BufRead};

fn main() {
    let mut eliza = eliza::Eliza::new();
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        match eliza.respond(&line) {
            eliza::Outcome::Say(text) => println!("? {line}\n{text}"),
            eliza::Outcome::ShutUp => {
                println!("? {line}\nShut up...");
                return;
            }
        }
    }
}
