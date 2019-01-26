#[macro_use]
extern crate log;
extern crate dotenv;
extern crate pretty_env_logger;
extern crate tokio_core;

mod config;
use chrono::prelude::*;
use chrono::Duration;
use egg_mode::tweet;
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

    let user_timeline =
        tweet::user_timeline(&user_info.id, true, true, &config.token, &handle).with_page_size(25);

    process_timeline("User Timeline", &mut core, user_timeline, config.preserve_days);

    let likes_timeline =
        tweet::liked_by(&user_info.id, &config.token, &handle).with_page_size(25);

    process_timeline("Likes Timeline", &mut core, likes_timeline, config.preserve_days);
}

fn process_timeline(name: &str, mut core: &mut Core, timeline: tweet::Timeline, preserve_days: i64) {
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

        process_timeline(name, &mut core, timeline, preserve_days);
    }
}
