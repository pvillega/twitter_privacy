#[macro_use]
extern crate log;

mod api;
mod config;

use api::{RealAPI, TwitterAPI};
use chrono::prelude::*;
use chrono::Duration;
use config::Config;
use config::EnvValues;
use egg_mode::tweet::Tweet;
use std::error::Error;
use tokio_core::reactor::Core;

/// Tries to erase old tweets for a user account
///
/// This method will load configuration from environment variables as described in the Readme file
/// and interact with that user account via Twitter API to erase tweets older than a configured
/// amount of days.
///
/// This of this method as a for comprehension with some side-effects when used in production
///
/// # Impure
///
/// - Loads values from environment variables
/// - Multiple requests to Twitter API
///
/// # Errors
///
/// - Configuration can't be loaded properly
/// - Errors while interacting with Twitter API
pub fn clear_old_tweets() -> Result<(), String> {
    // create the event loop that will drive this service, or fail if we can't
    info!("Initialise Tokio core");
    let core = match Core::new() {
        Ok(c) => c,
        Err(e) => return Err(e.description().to_string()),
    };

    info!("Retrieve environment values");
    let env_values = EnvValues::load()?;

    info!("Set up API trait for connecting to Twitter");
    let mut api = RealAPI {
        core: core,
        user_timeline: None,
        likes_timeline: None,
    };

    info!("Load configuration for the application");
    let config = config::Config::load(env_values, &mut api)?;
    // dbg!(&config);

    info!("Erase old Tweets for user");
    clear_user_timelines(&config, &mut api)
}

/// Processes a series of timelines for the given user to erase old tweets. The `Config` struct
/// contains the threshold for tweet deletion.
///
/// # Impure
///
/// - Multiple requests to Twitter API
///
/// # Errors
///
/// - Errors while removing elements from the timelines
/// - Other errors when interacting with Twitter API
fn clear_user_timelines<API: TwitterAPI>(config: &Config, api: &mut API) -> Result<(), String> {
    info!("Processing User timeline");
    let user_tl = || api.user_timeline_next_page(config.user_id, &config.token);
    process_timeline(
        "User Timeline",
        config.preserve_days,
        user_tl,
        default_maintenance_action,
    )?;

    info!("Processing Likes timeline");
    let likes_tl = || api.likes_timeline_next_page(config.user_id, &config.token);
    process_timeline(
        "Likes Timeline",
        config.preserve_days,
        likes_tl,
        default_maintenance_action,
    )?;

    info!("Processed all timelines. Exiting.");
    Ok(())
}

/// Given a function that returns a `Vector` of `Tweet`, it keeps calling the function and operation over
/// the elements returned until it reaches the end or an error is raised.
///
/// The default operation is that for any item older in days than the provided `preserve_days`, it will erase that element from the timeline.
///
/// # Impure
///
/// - Multiple requests to Twitter API
///
/// # Errors
///
/// - Errors while removing elements from the timelines
/// - Other errors when interacting with Twitter API
fn process_timeline<F, G>(
    name: &str,
    preserve_days: i64,
    mut tl_iterator: F,
    mut action: G,
) -> Result<(), String>
where
    F: FnMut() -> Result<Vec<Tweet>, String>,
    G: FnMut(&Tweet) -> Result<(), String>,
{
    let feed = tl_iterator()?;

    if feed.is_empty() {
        info!("We got to the end of the {} timeline", name);
        Ok(())
    } else {
        for tweet in &feed {
            if is_erasable(tweet.created_at, preserve_days) {
                action(tweet)?;
            }
        }

        process_timeline(name, preserve_days, tl_iterator, action)
    }
}

fn default_maintenance_action(tweet: &Tweet) -> Result<(), String> {
    //TODO trigger deletion in here
    info!(
        "Found ERASABLE <@{}> [{}] F:{}/RT:{} {}",
        tweet.user.as_ref().unwrap().screen_name,
        tweet.created_at,
        tweet.favorited.unwrap_or(false),
        tweet.retweeted.unwrap_or(false),
        tweet.text
    );
    Ok(())
}

/// Returns true if the given date is older (exclusively older!) in days than the value of `preserve_days`
fn is_erasable(created_at: DateTime<Utc>, preserve_days: i64) -> bool {
    let utc: DateTime<Utc> = Utc::now();
    utc.signed_duration_since(created_at) > Duration::days(preserve_days)
}

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

#[cfg(test)]
mod tests {
    mod clear_user_timeline {
        use crate::api::TestAPI;
        use crate::clear_user_timelines;
        use crate::config::Config;

        fn cfg() -> Config {
            let token = egg_mode::Token::Access {
                consumer: egg_mode::KeyPair::new("key", "secret"),
                access: egg_mode::KeyPair::new("key", "secret"),
            };

            Config {
                token,
                screen_name: String::from("screen_name"),
                user_id: 1,
                preserve_days: 10,
            }
        }

        #[test]
        fn propagates_errors_from_user_tl() {
            let mut api = TestAPI {
                user_timeline_next_page_answer: Err(String::from("bad answer")),
                ..Default::default()
            };

            assert_eq!(
                clear_user_timelines(&cfg(), &mut api),
                Err(String::from("bad answer"))
            )
        }

        #[test]
        fn propagates_errors_from_likes_tl() {
            let mut api = TestAPI {
                likes_timeline_next_page_answer: Err(String::from("bad answer")),
                ..Default::default()
            };

            assert_eq!(
                clear_user_timelines(&cfg(), &mut api),
                Err(String::from("bad answer"))
            )
        }

        #[test]
        fn calls_expected_methods_in_api() {
            let mut api = TestAPI {
                ..Default::default()
            };

            clear_user_timelines(&cfg(), &mut api).unwrap();

            let expected_calls = vec!["user_timeline_next_page", "likes_timeline_next_page"];

            assert_eq!(api.methods_called_in_order, expected_calls)
        }
    }
    mod process_timeline {
        use crate::api::sample_tweet;
        use crate::process_timeline;
        use egg_mode::tweet::Tweet;

        #[test]
        fn propagates_dataset_errors() {
            let dataset = || Err(String::from("Unexpected error"));
            let action = |_t: &Tweet| Ok(());

            assert_eq!(
                process_timeline("name", 1, dataset, action),
                Err(String::from("Unexpected error"))
            );
        }

        #[test]
        fn propagates_action_errors() {
            let tweet_vector = vec![sample_tweet(5)];

            let dataset = || Ok(tweet_vector.clone());
            let action = |_t: &Tweet| Err(String::from("Unexpected error"));

            assert_eq!(
                process_timeline("name", 1, dataset, action),
                Err(String::from("Unexpected error"))
            );
        }

        #[test]
        fn returns_ok_on_empty_dataset() {
            let dataset = || Ok(Vec::new());
            let action = |_t: &Tweet| Ok(());

            assert_eq!(process_timeline("name", 1, dataset, action), Ok(()));
        }

        quickcheck! {
            fn consumes_full_dataset(sz: usize) -> bool {
                let mut calls_made = 0;
                let mut tweet_vector = vec![sample_tweet(5); sz];

                let dataset = || {
                    match tweet_vector.pop() {
                        None => Ok(Vec::new()),
                        Some(v) => Ok(vec![v]),
                    }
                };
                let action = |_t: &Tweet| {
                    calls_made += 1;
                    Ok(())
                };

                process_timeline("name", 1, dataset, action).unwrap();

                calls_made == sz
            }
            fn only_calls_action_for_tweets_within_expected_time_window(oldsz: usize, newsz: usize) -> bool {
                let mut calls_made = 0;
                let mut old_vector = vec![sample_tweet(5); oldsz];
                let mut new_vector = vec![sample_tweet(2); newsz];
                old_vector.append(&mut new_vector);

                let dataset = || {
                    match old_vector.pop() {
                        None => Ok(Vec::new()),
                        Some(v) => Ok(vec![v]),
                    }
                };
                let action = |_t: &Tweet| {
                    calls_made += 1;
                    Ok(())
                };

                process_timeline("name", 4, dataset, action).unwrap();

                calls_made == oldsz
            }
        }
    }
    mod is_erasable {
        use crate::is_erasable;
        use chrono::prelude::*;

        quickcheck! {
            fn work_on_dates_as_expected(days_past: u32) -> bool {
                let now = Utc::now().timestamp();
                // not more than 10 years ago for testing purposes
                let bounded = (days_past % (365 * 10)) as i64;
                let seconds_past = bounded * 24 * 60 * 60;

                let dt = NaiveDateTime::from_timestamp(now - seconds_past, 0);
                let date = DateTime::from_utc(dt, Utc);

                // check the full range of date differences
                let mut boundary_after_date = true;
                for i in 0..bounded {
                    boundary_after_date = boundary_after_date && is_erasable(date, i);
                }

                let boundary_on_date = is_erasable(date, bounded);

                let mut boundary_before_date = false;
                for i in (bounded+1)..(bounded + 365) {
                    boundary_before_date = boundary_before_date && is_erasable(date, i);
                }
                boundary_after_date && boundary_on_date && !boundary_before_date
            }
        }
    }
}
