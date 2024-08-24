use std::{fs::{self, File}, io::Write, path::PathBuf};

use recently_used_xbel::update_recently_used;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("creating file");
    let mut file = File::create("foo.txt")?;
    file.write_all(b"Hello, world!")?;

    let path = PathBuf::from("./foo.txt");
    
    println!("canonicalized: {:?}", fs::canonicalize(&path));
    
    let path = fs::canonicalize(&path)?;

    let res = update_recently_used(&path);
    
    println!("res: {:?}", res);

    Ok(())
}
