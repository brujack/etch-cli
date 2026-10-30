fn main() {
    match jsonschemagen::render() {
        Ok(output) => print!("{output}"),
        Err(e) => {
            eprintln!("jsonschemagen: cannot serialize the manifest schema: {e}");
            std::process::exit(1);
        }
    }
}
