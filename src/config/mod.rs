mod env;

use crate::api::{APIError, TwitterAPI};
use egg_mode;
pub use env::EnvValues;

#[derive(Debug)]
pub struct Config {
    pub token: egg_mode::Token,
    pub screen_name: String,
    pub user_id: u64,
    pub preserve_days: i64,
}

impl Config {
    /// Uses a set of environment variables and a trait that provides access to the Twitter API
    /// to construct a configuration object, or return an error if that can't be done
    ///
    /// # Side Effects
    ///
    /// The `api` parameter may trigger calls to Twitter API
    ///
    /// # Error scenarios
    ///
    /// The method will return an `Err` if:
    ///
    /// - the values in `EnvValues` aren't valid tokens to interact with the API
    /// - the `api` parameter returns some error when we use its methods
    ///
    pub fn load<API: TwitterAPI>(env: EnvValues, api: &mut API) -> Result<Config, APIError> {
        info!("Creating configuraion object");

        let con_token = egg_mode::KeyPair::new(env.consumer_key, env.consumer_secret);
        let access_token = egg_mode::KeyPair::new(env.access_key, env.access_secret);
        let token = egg_mode::Token::Access {
            consumer: con_token,
            access: access_token,
        };

        // if not valid, short circuit to Err
        api.validate_token(&token)?;

        info!("Welcome back, {}!", &env.user_handle);

        let user_id = api.get_user_id(&env.user_handle, &token)?;

        let cfg = Config {
            token: token,
            screen_name: env.user_handle,
            user_id: user_id,
            preserve_days: env.preserve_days,
        };

        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::TestAPI;
    use quickcheck::{Arbitrary, Gen};
    use std::default::Default;

    impl Arbitrary for EnvValues {
        fn arbitrary<G: Gen>(g: &mut G) -> EnvValues {
            EnvValues {
                consumer_key: String::arbitrary(g),
                consumer_secret: String::arbitrary(g),
                access_key: String::arbitrary(g),
                access_secret: String::arbitrary(g),
                user_handle: String::arbitrary(g),
                preserve_days: i64::arbitrary(g),
            }
        }
    }

    fn sample_env_values() -> EnvValues {
        EnvValues {
            consumer_key: String::from("ck"),
            consumer_secret: String::from("cs"),
            access_key: String::from("ak"),
            access_secret: String::from("as"),
            user_handle: String::from("uh"),
            preserve_days: 1,
        }
    }

    #[test]
    fn error_if_invalid_token() {
        let err = APIError::InvalidToken;
        let mut api = TestAPI {
            validate_token_answer: Err(err.clone()),
            ..Default::default()
        };

        // can't use assert_eq on the result as Config can't implement PartialEq trait
        match Config::load(sample_env_values(), &mut api) {
            Ok(_) => panic!("It should return an error"),
            Err(e) => assert_eq!(e, err),
        }
    }

    #[test]
    fn error_if_api_user_id_fails() {
        let err = APIError::UserDetailsError(String::from("api error"));
        let mut api = TestAPI {
            get_user_id_answer: Err(err.clone()),
            ..Default::default()
        };

        // can't use assert_eq on the result as Config can't implement PartialEq trait
        match Config::load(sample_env_values(), &mut api) {
            Ok(_) => panic!("It should return an error"),
            Err(e) => assert_eq!(e, err),
        }
    }

    quickcheck! {
        fn config_has_expected_values(env_values: EnvValues, id: u64) -> bool {
            let mut api = TestAPI {
                get_user_id_answer: Ok(id),
            ..Default::default()
            };

            let config = Config::load(env_values.clone(), &mut api).unwrap();

            let id = config.user_id == id;
            let name = config.screen_name == env_values.user_handle;
            let days = config.preserve_days == env_values.preserve_days;
            let token = match config.token {
                egg_mode::Token::Bearer(_) => false,
                egg_mode::Token::Access{consumer, access} => {
                    consumer.key == env_values.consumer_key &&
                    consumer.secret == env_values.consumer_secret &&
                    access.key == env_values.access_key &&
                    access.secret == env_values.access_secret
                },
            };

            id && name && days && token
        }
    }
}
