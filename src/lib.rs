#[macro_use]
extern crate log;

mod api;
mod config;

use api::{APIError, RealAPI, TwitterAPI};
use chrono::prelude::*;
use chrono::Duration;
use config::EnvValues;
use egg_mode::tweet::Tweet;
use std::fmt;

/// Defines errors we can get when executing the methods of the library
#[derive(Debug, Clone, PartialEq)]
pub enum Errors {
    APIErrors(APIError),
    EnvValueErrors(String),
    LibErrors(String),
}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Errors::APIErrors(s) => write!(f, "Error interacting with Twitter API: {}", s),
            Errors::EnvValueErrors(s) => write!(f, "Error reading environment variables: {}", s),
            Errors::LibErrors(s) => write!(f, "Error: {}", s),
        }
    }
}

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
pub fn clear_old_tweets() -> Result<(), Errors> {
    info!("Retrieve environment values");
    let env_values = EnvValues::load().map_err(Errors::EnvValueErrors)?;
    let preserve_days = env_values.preserve_days;
    // dbg!(&env_values);

    info!("Set up API trait for connecting to Twitter");
    let mut api = RealAPI::new(env_values).map_err(Errors::APIErrors)?;

    info!("Erase old Tweets for user");
    clear_user_timelines(&mut api, preserve_days)
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
fn clear_user_timelines(api: &mut dyn TwitterAPI, preserve_days: i64) -> Result<(), Errors> {
    info!("Processing User timeline");
    let user_tl = |c_api: &mut dyn TwitterAPI| c_api.user_timeline_next_page();
    process_timeline(
        "User Timeline",
        preserve_days,
        api,
        user_tl,
        default_maintenance_action,
    )?;

    info!("Processing Likes timeline");
    let likes_tl = |c_api: &mut dyn TwitterAPI| c_api.likes_timeline_next_page();
    process_timeline(
        "Likes Timeline",
        preserve_days,
        api,
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
fn process_timeline<'a, F, G>(
    name: &str,
    preserve_days: i64,
    api: &mut dyn TwitterAPI,
    mut tl_iterator: F,
    mut action: G,
) -> Result<(), Errors>
where
    F: FnMut(&mut dyn TwitterAPI) -> Result<Vec<Tweet>, APIError>,
    G: FnMut(&mut dyn TwitterAPI, &Tweet) -> Result<(), Errors> + 'a,
{
    let feed = tl_iterator(api).map_err(Errors::APIErrors)?;

    if feed.is_empty() {
        info!("We got to the end of the {} timeline", name);
        Ok(())
    } else {
        info!("Processing next page of {} timeline", name);
        for tweet in &feed {
            if is_erasable(tweet.created_at, preserve_days) {
                action(api, tweet)?;
            }
        }

        process_timeline(name, preserve_days, api, tl_iterator, action)
    }
}

fn default_maintenance_action(api: &mut dyn TwitterAPI, tweet: &Tweet) -> Result<(), Errors> {
    warn!(
        "Erasing tweet created at: [{}] - F:{}|RT:{} -- {}",
        tweet.created_at,
        tweet.favorited.unwrap_or(false),
        tweet.retweeted.unwrap_or(false),
        tweet.text
    );

    if tweet.favorited.unwrap_or(false) {
        api.unlike_tweet(&tweet).map_err(Errors::APIErrors)?;
    }
    if tweet.retweeted.unwrap_or(false) {
        api.unretweet_tweet(&tweet).map_err(Errors::APIErrors)?;
    }

    api.erase_tweet(&tweet).map_err(Errors::APIErrors)
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
    use chrono::prelude::*;
    use egg_mode::tweet::{Tweet, TweetEntities, TweetSource};

    pub fn sample_tweet(days_ago: i64) -> Tweet {
        let now = Utc::now().timestamp();
        let seconds_past = days_ago * 24 * 60 * 60;
        let dt = NaiveDateTime::from_timestamp(now - seconds_past, 0);
        let date = DateTime::from_utc(dt, Utc);
        Tweet {
            coordinates: None,
            created_at: date,
            current_user_retweet: None,
            display_text_range: None,
            entities: TweetEntities {
                hashtags: Vec::new(),
                symbols: Vec::new(),
                urls: Vec::new(),
                user_mentions: Vec::new(),
                media: None,
            },
            extended_entities: None,
            favorite_count: 20,
            favorited: None,
            filter_level: None,
            id: 1,
            in_reply_to_user_id: None,
            in_reply_to_screen_name: None,
            in_reply_to_status_id: None,
            lang: Some(String::from("und")),
            place: None,
            possibly_sensitive: None,
            quoted_status_id: None,
            quoted_status: None,
            retweet_count: 10,
            retweeted: None,
            retweeted_status: None,
            source: TweetSource {
                name: String::from("source name"),
                url: String::from("source url"),
            },
            text: String::from("a sample tweet"),
            truncated: false,
            user: None,
            withheld_copyright: false,
            withheld_in_countries: None,
            withheld_scope: None,
        }
    }

    mod clear_user_timeline {
        use crate::api::{APIError, TestAPI};
        use crate::clear_user_timelines;
        use crate::Errors;

        #[test]
        fn propagates_errors_from_user_tl() {
            let err = APIError::TimelineError(String::from("bad answer"));
            let mut api = TestAPI {
                user_timeline_next_page_answer: Err(err.clone()),
                ..Default::default()
            };

            assert_eq!(
                clear_user_timelines(&mut api, 10),
                Err(Errors::APIErrors(err))
            )
        }

        #[test]
        fn propagates_errors_from_likes_tl() {
            let err = APIError::TimelineError(String::from("bad answer"));
            let mut api = TestAPI {
                likes_timeline_next_page_answer: Err(err.clone()),
                ..Default::default()
            };

            assert_eq!(
                clear_user_timelines(&mut api, 10),
                Err(Errors::APIErrors(err))
            )
        }

        #[test]
        fn calls_expected_methods_in_api() {
            let mut api = TestAPI {
                ..Default::default()
            };

            clear_user_timelines(&mut api, 10).unwrap();

            let expected_calls = vec!["user_timeline_next_page", "likes_timeline_next_page"];

            assert_eq!(api.methods_called_in_order, expected_calls)
        }
    }
    mod process_timeline {
        use super::sample_tweet;
        use crate::api::{APIError, TestAPI, TwitterAPI};
        use crate::process_timeline;
        use crate::Errors;
        use egg_mode::tweet::Tweet;

        #[test]
        fn propagates_dataset_errors() {
            let mut api = TestAPI {
                ..Default::default()
            };
            let err = APIError::TimelineError(String::from("Unexpected error"));
            let dataset = |_a: &mut dyn TwitterAPI| Err(err.clone());
            let action = |_a: &mut dyn TwitterAPI, _t: &Tweet| Ok(());

            assert_eq!(
                process_timeline("name", 1, &mut api, dataset, action),
                Err(Errors::APIErrors(err))
            );
        }

        #[test]
        fn propagates_action_errors() {
            let mut api = TestAPI {
                ..Default::default()
            };
            let tweet_vector = vec![sample_tweet(5)];
            let err = Errors::LibErrors(String::from("Unexpected error"));

            let dataset = |_a: &mut dyn TwitterAPI| Ok(tweet_vector.clone());
            let action = |_a: &mut dyn TwitterAPI, _t: &Tweet| Err(err.clone());

            assert_eq!(
                process_timeline("name", 1, &mut api, dataset, action),
                Err(err)
            );
        }

        #[test]
        fn returns_ok_on_empty_dataset() {
            let mut api = TestAPI {
                ..Default::default()
            };
            let dataset = |_a: &mut dyn TwitterAPI| Ok(Vec::new());
            let action = |_a: &mut dyn TwitterAPI, _t: &Tweet| Ok(());

            assert_eq!(
                process_timeline("name", 1, &mut api, dataset, action),
                Ok(())
            );
        }

        quickcheck! {
            fn consumes_full_dataset(sz: usize) -> bool {
                let mut api = TestAPI{..Default::default()};
                let mut calls_made = 0;
                let mut tweet_vector = vec![sample_tweet(5); sz];

                let dataset = |_a: &mut dyn TwitterAPI| {
                    match tweet_vector.pop() {
                        None => Ok(Vec::new()),
                        Some(v) => Ok(vec![v]),
                    }
                };
                let action = |_a: &mut dyn TwitterAPI, _t: &Tweet| {
                    calls_made += 1;
                    Ok(())
                };

                process_timeline("name", 1, &mut api, dataset,  action).unwrap();

                calls_made == sz
            }
            fn only_calls_action_for_tweets_within_expected_time_window(oldsz: usize, newsz: usize) -> bool {
                let mut api = TestAPI{..Default::default()};
                let mut calls_made = 0;
                let mut old_vector = vec![sample_tweet(5); oldsz];
                let mut new_vector = vec![sample_tweet(2); newsz];
                old_vector.append(&mut new_vector);

                let dataset = |_a: &mut dyn TwitterAPI| {
                    match old_vector.pop() {
                        None => Ok(Vec::new()),
                        Some(v) => Ok(vec![v]),
                    }
                };
                let action = |_a: &mut dyn TwitterAPI, _t: &Tweet| {
                    calls_made += 1;
                    Ok(())
                };

                process_timeline("name", 4, &mut api, dataset, action).unwrap();

                calls_made == oldsz
            }
        }
    }

    mod default_maintenance_action {
        use super::sample_tweet;
        use crate::api::{APIError, TestAPI};
        use crate::default_maintenance_action;
        use crate::Errors;

        #[test]
        fn propagates_unlike_api_errors() {
            let err = APIError::ErasureError(String::from("Unexpected error"));
            let mut api = TestAPI {
                unlike_tweet_answer: Err(err.clone()),
                ..Default::default()
            };

            let mut tweet = sample_tweet(1);
            tweet.favorited = Some(true);

            assert_eq!(
                default_maintenance_action(&mut api, &tweet),
                Err(Errors::APIErrors(err))
            );
        }

        #[test]
        fn propagates_unretweet_api_errors() {
            let err = APIError::ErasureError(String::from("Unexpected error"));
            let mut api = TestAPI {
                unretweet_tweet_answer: Err(err.clone()),
                ..Default::default()
            };

            let mut tweet = sample_tweet(1);
            tweet.retweeted = Some(true);

            assert_eq!(
                default_maintenance_action(&mut api, &tweet),
                Err(Errors::APIErrors(err))
            );
        }

        #[test]
        fn propagates_erase_api_errors() {
            let err = APIError::ErasureError(String::from("Unexpected error"));
            let mut api = TestAPI {
                erase_tweet_answer: Err(err.clone()),
                ..Default::default()
            };

            assert_eq!(
                default_maintenance_action(&mut api, &sample_tweet(1)),
                Err(Errors::APIErrors(err))
            );
        }

        #[test]
        fn calls_expected_methods_if_liked() {
            let mut api = TestAPI {
                ..Default::default()
            };

            let mut tweet = sample_tweet(1);
            tweet.favorited = Some(true);

            default_maintenance_action(&mut api, &tweet).unwrap();

            let expected = vec!["unlike_tweet", "erase_tweet"];
            assert_eq!(api.methods_called_in_order, expected);
        }

        #[test]
        fn calls_expected_methods_if_retweeted() {
            let mut api = TestAPI {
                ..Default::default()
            };

            let mut tweet = sample_tweet(1);
            tweet.retweeted = Some(true);

            default_maintenance_action(&mut api, &tweet).unwrap();

            let expected = vec!["unretweet_tweet", "erase_tweet"];
            assert_eq!(api.methods_called_in_order, expected);
        }

        #[test]
        fn calls_expected_methods_if_all() {
            let mut api = TestAPI {
                ..Default::default()
            };

            let mut tweet = sample_tweet(1);
            tweet.favorited = Some(true);
            tweet.retweeted = Some(true);

            default_maintenance_action(&mut api, &tweet).unwrap();

            let expected = vec!["unlike_tweet", "unretweet_tweet", "erase_tweet"];
            assert_eq!(api.methods_called_in_order, expected);
        }
    }
    mod is_erasable {
        use crate::is_erasable;
        use chrono::prelude::*;

        quickcheck! {
            fn work_on_dates_as_expected(days_past: u32) -> bool {
                let now = Utc::now().timestamp();
                // not more than 10 years ago for testing purposes
                let bounded = i64::from(days_past % (365 * 10));
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
