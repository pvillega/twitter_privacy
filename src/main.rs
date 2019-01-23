#[macro_use]
extern crate log;
extern crate dotenv;
extern crate pretty_env_logger;
extern crate tokio_core;

mod config;
use tokio_core::reactor::Core;

fn main() {
    // first load .env values to env::var
    dotenv::dotenv().ok();

    // then set-up the logger as it will use env::vars for init
    if let Err(e) = pretty_env_logger::try_init() {
        eprintln!("Error initialising `pretty_env_logger` {}", e);
        panic!("Missing logger. Aborting!")
    };

    // Create the event loop that will drive this server
    let mut core = Core::new().unwrap();
    let config = config::Config::load(&mut core);
    // dbg!(&config);
    
    let handle = core.handle();


    let user_info = core
        .run(egg_mode::user::show(
            &config.screen_name,
            &config.token,
            &handle,
        ))
        .unwrap();

    info!("{} (@{})", user_info.name, user_info.screen_name);
}
