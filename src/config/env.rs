use std::env;
use std::env::VarError;

/// List of values that we will need to interact with Twitter.
/// Intended to be used to build our Configuration structs
///
/// It is extracted as an additional object instead of being part of our configuration
/// to facilitate testing
#[derive(Debug, Clone)]
pub struct EnvValues {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub access_key: String,
    pub access_secret: String,
    pub user_handle: String,
    pub preserve_days: i64,
}

impl EnvValues {
    // list of environment variables we will load
    const CONSUMER_KEY: &'static str = "TP_CONSUMER_KEY";
    const CONSUMER_SECRET: &'static str = "TP_CONSUMER_SECRET";
    const ACCESS_KEY: &'static str = "TP_ACCESS_KEY";
    const ACCESS_SECRET: &'static str = "TP_ACCESS_SECRET";
    const USER_HANDLE: &'static str = "TP_USER_HANDLE";
    const PRESERVE_DAYS: &'static str = "TP_PRESERVE_DAYS";

    /// Loads a set of environmnt variables into a `EnvValues` struct
    ///
    /// # Side effects
    /// 
    /// Reads from environment variables
    /// 
    /// # Error scenarios
    ///
    /// The method will return an Err(_) if:
    ///
    /// - any of the needed environment variables is missing, or the wrong format
    pub fn load() -> Result<EnvValues, String> {
        info!("Loading environment variables and parsing to proper types");
        
        //We load configuration from environment. Fail early (using ?) if something is wrong
        let consumer_key = EnvValues::get_env_var(EnvValues::CONSUMER_KEY)?;
        let consumer_secret = EnvValues::get_env_var(EnvValues::CONSUMER_SECRET)?;
        let access_key = EnvValues::get_env_var(EnvValues::ACCESS_KEY)?;
        let access_secret = EnvValues::get_env_var(EnvValues::ACCESS_SECRET)?;
        let user_handle = EnvValues::get_env_var(EnvValues::USER_HANDLE)?;

        let preserve_days = EnvValues::get_env_var(EnvValues::PRESERVE_DAYS)?;
        // on this code (parse()) the macro try! or the shortcut '?' break inference, so we need to unroll them
        let preserve_days: i64 = match preserve_days.parse::<i64>() {
            Ok(i) => i,
            Err(e) => {
                return Err(format!(
                    "Error parsing {} to an i64: {}",
                    EnvValues::PRESERVE_DAYS,
                    e
                ));
            }
        };

        Ok(EnvValues {
            consumer_key,
            consumer_secret,
            access_key,
            access_secret,
            user_handle,
            preserve_days,
        })
    }

    // loads the environment variable with the given name
    fn get_env_var(name: &str) -> Result<String, String> {
        let map_if_err = EnvValues::varerror_to_string(String::from(name));
        env::var(name).map_err(map_if_err)
    }

    // used to map VarError to Strings with the corresponding message
    fn varerror_to_string(name: String) -> impl Fn(VarError) -> String {
        move |v| match v {
            VarError::NotPresent => format!("Environment variable {:?} not found", name),
            VarError::NotUnicode(s) => format!(
                "Environment variable {:?} was not valid unicode: {:?}",
                name, s
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    // These tests are quite useless, just added to play around with QuickCheck
    quickcheck! {
        fn for_not_present(n: String) -> bool {
            let expected = format!("Environment variable {:?} not found", &n);
            EnvValues::varerror_to_string(n)(VarError::NotPresent) == expected
        }

        fn for_not_unicode(n: String, s: String) -> bool {
            let expected = format!("Environment variable {:?} was not valid unicode: {:?}", &n, &s);
            EnvValues::varerror_to_string(n)(VarError::NotUnicode(OsString::from(s))) == expected
        }
    }
}
