extern crate pretty_env_logger;
#[macro_use]
extern crate log;
extern crate dotenv;

use std::env;

fn main() {
    // first load .env values to env::var
    dotenv::dotenv().ok();
    // then set-up the logger as it will use env::vars for init
    pretty_env_logger::init();

    for (key, value) in env::vars() {
        if key == "TP_CONSUMER_KEY"
            || key == "TP_CONSUMER_SECRET"
            || key == "TP_ACCESS_KEY"
            || key == "TP_ACCESS_SECRET"
            || key == "TP_USER_HANDLE"
        {
            warn!("{}: {}", key, value);
        } else {
            info!("{}: {}", key, value);
        }
    }
}
