use std::time::Duration;

use grafton_ndi::{Finder, FinderOptions, NDI};

fn main() {
    let ndi = match NDI::new() {
        Ok(ndi) => ndi,
        Err(error) => {
            eprintln!("NDI init failed: {error}");
            std::process::exit(1);
        }
    };

    let finder = match Finder::new(
        &ndi,
        &FinderOptions::builder().show_local_sources(true).build(),
    ) {
        Ok(finder) => finder,
        Err(error) => {
            eprintln!("NDI finder failed: {error}");
            std::process::exit(1);
        }
    };

    if let Err(error) = finder.wait_for_sources(Duration::from_secs(2)) {
        eprintln!("NDI discovery wait failed: {error}");
        std::process::exit(1);
    }

    let sources = match finder.current_sources() {
        Ok(sources) => sources,
        Err(error) => {
            eprintln!("NDI source query failed: {error}");
            std::process::exit(1);
        }
    };

    println!("NDI initialized successfully");
    println!("Discovered {} source(s)", sources.len());
    for source in sources {
        println!("- {}", source.name);
    }
}
