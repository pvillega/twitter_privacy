#[macro_use]
extern crate log;

mod config;

use chrono::prelude::*;
use chrono::Duration;
use config::Config;
use egg_mode::tweet;
use egg_mode::tweet::Timeline;
use tokio_core::reactor::Core;

pub fn clear_old_tweets(core: &mut Core) {
    // load configuration for
    let config = config::Config::load(core);
    // dbg!(&config);

    clear_user_timelines(&config, core);
}

fn clear_user_timelines(config: &Config, core: &mut Core) {
    let handle = core.handle();

    let user_info = core
        .run(egg_mode::user::show(
            &config.screen_name,
            &config.token,
            &handle,
        ))
        .unwrap();

    info!("{} (@{})", user_info.name, user_info.screen_name);

    let user_timeline =
        tweet::user_timeline(&user_info.id, true, true, &config.token, &handle).with_page_size(25);

    process_timeline("User Timeline", core, user_timeline, config.preserve_days);

    let likes_timeline = tweet::liked_by(&user_info.id, &config.token, &handle).with_page_size(25);

    process_timeline("Likes Timeline", core, likes_timeline, config.preserve_days);
}

fn process_timeline(name: &str, core: &mut Core, timeline: Timeline, preserve_days: i64) {
    let future_timeline = timeline.older(None);
    let (timeline, feed) = core.run(future_timeline).unwrap();

    if feed.is_empty() {
        info!("We got to the end of the {} timeline", name);
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

        process_timeline(name, core, timeline, preserve_days);
    }
}
