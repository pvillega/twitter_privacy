use egg_mode;
use std::env;
use tokio_core::reactor::Core;

#[derive(Debug)]
pub struct Config {
    pub token: egg_mode::Token,
    pub screen_name: String,
}

impl Config {
    pub fn load(core: &mut Core) -> Self {
        //We load configuration from environment. We use unwrap here as we want to panic if something is missing
        let consumer_key = env::var("TP_CONSUMER_KEY").unwrap();
        let consumer_secret = env::var("TP_CONSUMER_SECRET").unwrap();
        let access_key = env::var("TP_ACCESS_KEY").unwrap();
        let access_secret = env::var("TP_ACCESS_SECRET").unwrap();
        let username = env::var("TP_USER_HANDLE").unwrap();

        let con_token = egg_mode::KeyPair::new(consumer_key, consumer_secret);
        let access_token = egg_mode::KeyPair::new(access_key, access_secret);

        let token = egg_mode::Token::Access {
            consumer: con_token,
            access: access_token,
        };

        let handle = core.handle();

        if let Err(err) = core.run(egg_mode::verify_tokens(&token, &handle)) {
            println!("We've hit an error using your tokens: {:?}", err);
            panic!("Invalid tokens, the application can't continue.")
        } else {
            info!("Welcome back, {}!", username);
            Config {
                token: token,
                screen_name: username,
            }
        }
    }
}
