use std::io::{self, BufRead};

// use log::{info, debug};
use env_logger::Env;

use heli_otr::identifier::Identifier;


fn run(mut identifier: Identifier) {
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        println!("{}", identifier.identify(&line.unwrap()).0);
    }
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let identifier = Identifier::new(String::from("gramdict.ser"),
                                     String::from("wordict.ser"));
    run(identifier);
}
