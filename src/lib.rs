#[macro_use]
extern crate log;

mod config;

use chrono::prelude::*;
use chrono::Duration;
use config::Config;
use egg_mode::tweet;
use egg_mode::tweet::Timeline;
use std::error::Error;
use tokio_core::reactor::Core;

/// Tries to erase old tweets for a user account
///
/// This method will load configuration from environment variables as described in Readme
/// and interact with that user account via Twitter API to erase tweets older than a configured
/// amount of days.
///
/// The logic for loading configuration is purposedly included in this call so we can propagate
/// all errors to the caller
/// 
/// # Impure
/// 
/// Multiple requests to Twitter API
/// 
/// # Errors
/// 
/// - Configuration can't be loaded properly
/// - Errors while interacting with Twitter API
pub fn clear_old_tweets(core: &mut Core) -> Result<(), String> {
    info!("Load configuration");
    let config = config::Config::load(core)?;
    // dbg!(&config);

    info!("Get Twitter's user id for selected used");
    let user_id = get_user_id(&config, core)?;

    info!("Erase old Tweets for user");
    clear_user_timelines(user_id, &config, core);

    Ok(())
}

/// Queries Twitter to obtain the user id associated with the user for which we provide the tokens,
/// contained within the `Config` struct
/// 
/// # Impure
/// 
/// Sends a request to Twitter API to get user information
/// 
/// # Errors
/// 
/// Method will return an error if the request to Twitter fails for any reason
fn get_user_id(config: &Config, core: &mut Core) -> Result<u64, String> {
    let handle = core.handle();

    let query_twitter = core.run(egg_mode::user::show(
        &config.screen_name,
        &config.token,
        &handle,
    ));

    let user_info = match query_twitter {
        Ok(uinfo) => uinfo,
        Err(e) => return Err(e.description().to_string()),
    };

    info!(
        "Retrieved user id for {} (@{})",
        user_info.name, user_info.screen_name
    );

    Ok(user_info.id)
}

/// Processes a series of timelines for the given user to erase old tweets. The `Config` struct 
/// contains the threshold for tweet deletion.
/// 
/// # Impure
/// 
/// Calls to Twitter API
/// 
/// # Errors
/// 
/// Errors while clearing any of the timelines
fn clear_user_timelines(user_id: u64, config: &Config, core: &mut Core) -> Result<(), String> {
    let handle = core.handle();

    let user_timeline =
        tweet::user_timeline(user_id, true, true, &config.token, &handle).with_page_size(25);

    process_timeline("User Timeline", core, user_timeline, config.preserve_days)?;

    let likes_timeline = tweet::liked_by(user_id, &config.token, &handle).with_page_size(25);

    process_timeline("Likes Timeline", core, likes_timeline, config.preserve_days)
}

/// Given a `Timeline` struct (an object that references a Twitter timeline, including our current position in it)
/// it iterates over that timeline until we reach the end.
/// 
/// For any item older in days than the provided `preserve_days`, it will erase that element from the timeline.
/// 
/// # Impure
/// 
/// Calls to Twitter API
/// 
/// # Errors
/// 
/// Errors while interacting with Twitter API
fn process_timeline(name: &str, core: &mut Core, timeline: Timeline, preserve_days: i64) -> Result<(), String> {
    let future_timeline = timeline.older(None);
    let (timeline, feed) = core.run(future_timeline).unwrap();

    if feed.is_empty() {
        info!("We got to the end of the {} timeline", name);
        Ok(())
    } else {
        let utc: DateTime<Utc> = Utc::now();
        for tweet in &feed {
            if utc.signed_duration_since(tweet.created_at) > Duration::days(preserve_days) {
                info!(
                    "<@{}> [{}] F:{}/RT:{} {}",
                    tweet.user.as_ref().unwrap().screen_name,
                    tweet.created_at,
                    tweet.favorited.unwrap_or(false),
                    tweet.retweeted.unwrap_or(false),
                    tweet.text
                );
            }
        }

        process_timeline(name, core, timeline, preserve_days)
    }
}
