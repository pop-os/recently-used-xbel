fn main() -> Result<(), Box<dyn std::error::Error>> {
    let recently_used = recently_used_xbel::parse_file()?;

    for bookmark in recently_used.bookmarks {
        println!("{:#?}", bookmark);
    }

    Ok(())
}