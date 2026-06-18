use tandem_client_lib::capture::list_all_sources;

fn main() {
    match list_all_sources() {
        Ok(sources) => {
            if sources.is_empty() {
                eprintln!("list-sources-probe: no sources found");
                std::process::exit(1);
            }
            for source in sources {
                println!("{} {} {}", source.id, format!("{:?}", source.kind), source.label);
            }
        }
        Err(error) => {
            eprintln!("list-sources-probe: {error}");
            std::process::exit(1);
        }
    }
}
