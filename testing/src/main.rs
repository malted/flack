use std::env::args;
use std::fs::File;
use std::path::Path;
use flack::{lock_file, unlock_file};

fn main() {
	let path = args().nth(2).expect("invalid args. the second one should be a path.");
	let path = Path::new(&path);
	let file = File::open(path).expect("flock failed to open file");

	match args().nth(1).unwrap().as_str() {
		"lock" => lock_file(&file).expect("failed to lock file"),
		"unlock" => unlock_file(&file).expect("failed to unlock file"),
		_ => panic!("invalid argument. should either be \"lock\" or \"unlock\"")
	}

	loop { }
}
