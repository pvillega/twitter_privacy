// we use our own crate as external to access the library methods
extern crate twitter_privacy;

#[macro_use]
extern crate log;
extern crate dotenv;
extern crate pretty_env_logger;
extern crate tokio_core;

use tokio_core::reactor::Core;

fn main() {
    // first load .env values to env::var
    dotenv::dotenv().ok();

    // then set-up the logger as it will use env::vars for initialisation. Panic if we can't do so.
    if let Err(e) = pretty_env_logger::try_init() {
        eprintln!("Error initialising `pretty_env_logger` {}", e);
        panic!("Missing logger. Aborting!")
    };
    
    // create the event loop that will drive this service, or panic if we can't
    let mut core = Core::new().unwrap();

    // call method to clean old tweets. All the logic happens in the lib. We receive a Result and exit accordingly.
    match twitter_privacy::clear_old_tweets(&mut core) {
        Ok(_) => info!("Tweets erased, stopping process. Thanks for using this application!"),
        Err(e) =>{
            error!("There's been an error:\n {}", e);
            panic!("Unrecoverable error while trying to erase Tweets. Aborting!")
        },
    };
}


