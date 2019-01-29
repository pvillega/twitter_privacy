use egg_mode;
use std::env;
use std::env::VarError;
use tokio_core::reactor::Core;

#[derive(Debug)]
pub struct Config {
    pub token: egg_mode::Token,
    pub screen_name: String,
    pub preserve_days: i64,
}

impl Config {
    // list of environment variables we will load
    const CONSUMER_KEY : &'static str = "TP_CONSUMER_KEY";
    const CONSUMER_SECRET : &'static str = "TP_CONSUMER_SECRET";
    const ACCESS_KEY : &'static str = "TP_ACCESS_KEY";
    const ACCESS_SECRET : &'static str = "TP_ACCESS_SECRET";
    const USER_HANDLE : &'static str = "TP_USER_HANDLE";
    const PRESERVE_DAYS : &'static str = "TP_PRESERVE_DAYS";

    /// Reads a set of environment variables and returns a `Config` object
    /// which contains credentials that can be used with the Twitter API
    /// 
    /// # Impure
    /// 
    /// The method does a connection to Twitter to verify the values in the 
    /// environment variables correspond to valid tokens.
    /// 
    /// # Error scenarios
    /// 
    /// The method will return an Err(_) if:
    /// 
    /// - any of the needed environment variables is missing or the wrong format
    /// - the token for the user (any of consumer or access keys and secrets) are invalid and Twitter rejects them
    /// 
    pub fn load(core: &mut Core) -> Result<Self, String> {
        //We load configuration from environment. Fail early (using ?) if something is wrong
        let consumer_key = Config::get_env_var(Config::CONSUMER_KEY)?;
        let consumer_secret = Config::get_env_var(Config::CONSUMER_SECRET)?;
        let access_key = Config::get_env_var(Config::ACCESS_KEY)?;
        let access_secret = Config::get_env_var(Config::ACCESS_SECRET)?;
        let username = Config::get_env_var(Config::USER_HANDLE)?;
        let preserve_days = Config::get_env_var(Config::PRESERVE_DAYS)?;
        // on this code (parse()) the macro try! or the shortcut '?' break inference, so we need to unroll them
        let preserve_days: i64 = match preserve_days.parse::<i64>() {
            Ok(i) => i,
            Err(e) => return Err(format!("Error parsing {} to an i64: {}", Config::PRESERVE_DAYS, e)),
        };

        let con_token = egg_mode::KeyPair::new(consumer_key, consumer_secret);
        let access_token = egg_mode::KeyPair::new(access_key, access_secret);
        let token = egg_mode::Token::Access {
            consumer: con_token,
            access: access_token,
        };

        let handle = core.handle();

        if let Err(err) = core.run(egg_mode::verify_tokens(&token, &handle)) {
            let msg = format!("We've hit an error using your tokens: {:?}. Invalid tokens, the application can't continue.", err);
            Err(msg)
        } else {
            info!("Welcome back, {}!", username);
            let cfg = Config {
                token: token,
                screen_name: username,
                preserve_days: preserve_days,
            };
            Ok(cfg)
        }
    }

    fn get_env_var(name: &'static str) -> Result<String, String> {
        let map_if_err = Config::varerror_to_string(name);
        env::var(name).map_err(map_if_err)
    }

    fn varerror_to_string(name: &'static str) -> impl Fn(VarError) -> String {
        move |v| match v {
            VarError::NotPresent => format!("Environment variable {:?} not found", name),
            VarError::NotUnicode(s) => {
                format!("Environment variable {:?} was not valid unicode: {:?}", name, s)
            }
        }
    }
}
