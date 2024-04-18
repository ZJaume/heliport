use std::io::{self, BufRead};

use log::{info, debug};

use heli_otr::identifier::Identifier;


fn main() {
    let mut identifier = Identifier::new(String::from("gramdict.ser"),
                                     String::from("wordict.ser"));

    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        println!("{}", identifier.identify(&line.unwrap()).0);
    }
}
