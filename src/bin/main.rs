// we use our own crate as external to access the library methods
extern crate twitter_privacy;

extern crate dotenv;
extern crate pretty_env_logger;
extern crate tokio_core;

use tokio_core::reactor::Core;

fn main() {
    // first load .env values to env::var
    dotenv::dotenv().ok();

    // then set-up the logger as it will use env::vars for initialisation
    if let Err(e) = pretty_env_logger::try_init() {
        eprintln!("Error initialising `pretty_env_logger` {}", e);
        panic!("Missing logger. Aborting!")
    };
    
    // Create the event loop that will drive this service, or panic if we can't
    let mut core = Core::new().unwrap();

    twitter_privacy::clear_old_tweets(&mut core);
}


