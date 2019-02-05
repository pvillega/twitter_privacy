use egg_mode;
use egg_mode::tweet;
use egg_mode::tweet::{Timeline, Tweet};
use std::error::Error;
use tokio_core::reactor::Core;
use std::fmt;

/// Defines errors that can happen when calling the API methods
#[derive(Debug, Clone, PartialEq)]
pub enum APIError {
    InvalidToken,
    TimelineError(String),
    UserDetailsError(String)
}

impl fmt::Display for APIError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            APIError::InvalidToken => write!(f, "Invalid or Expired Token used in the request to Twitter API"),
            APIError::TimelineError(s) => write!(f, "Error when retrieving data from a timeline: {}", s),
            APIError::UserDetailsError(s) => write!(f, "Failure obtaining user details: {}", s),
        }
    }
}

/// Trait that defines methods we need to interact with the Twitter API
/// Created so we can avoid real API calls during testing, using a stub instead
pub trait TwitterAPI {
    /// Verifies the given token is valid, otherwise returns an Err result.
    fn validate_token(&mut self, token: &egg_mode::Token) -> Result<(), APIError>;

    /// Returns the unique user id (an u64) for the user with the given screen name
    fn get_user_id(&mut self, screen_name: &str, token: &egg_mode::Token)
        -> Result<u64, APIError>;

    /// Returns the next page available of user timeline for given user id, which contains tweets published (or retweeted) by user
    fn user_timeline_next_page(
        &mut self,
        user_id: u64,
        token: &egg_mode::Token,
    ) -> Result<Vec<Tweet>, APIError>;

    /// Returns the next page available of tweets liked by given user id
    fn likes_timeline_next_page(
        &mut self,
        user_id: u64,
        token: &egg_mode::Token,
    ) -> Result<Vec<Tweet>, APIError>;
}

/// Struct that has an implementation of TwitterAPI that calls twitter servers
pub struct RealAPI<'a> {
    pub core: Core,
    pub user_timeline: Option<Timeline<'a>>,
    pub likes_timeline: Option<Timeline<'a>>,
}

impl<'a> TwitterAPI for RealAPI<'a> {
    fn validate_token(&mut self, token: &egg_mode::Token) -> Result<(), APIError> {
        info!("Verifying validity of Token by querying Twitter API");
        let handle = self.core.handle();

        if let Err(err) = self.core.run(egg_mode::verify_tokens(token, &handle)) {
            error!("We've hit an error using your tokens: {:?}. Invalid tokens, the application can't continue.", err);
            Err(APIError::InvalidToken)
        } else {
            info!("Tokens seem to be valid");
            Ok(())
        }
    }

    fn get_user_id(
        &mut self,
        screen_name: &str,
        token: &egg_mode::Token,
    ) -> Result<u64, APIError> {
        info!("Requesting user id for user {}", screen_name);
        let handle = self.core.handle();

        let query_for_user = self
            .core
            .run(egg_mode::user::show(screen_name, token, &handle));

        let user_info = match query_for_user {
            Ok(uinfo) => uinfo,
            Err(e) => return Err(APIError::UserDetailsError(e.description().to_string())),
        };

        info!(
            "Retrieved user id {} for {} (@{})",
            user_info.id, user_info.name, user_info.screen_name
        );

        Ok(user_info.id)
    }

    fn user_timeline_next_page(
        &mut self,
        user_id: u64,
        token: &egg_mode::Token,
    ) -> Result<Vec<Tweet>, APIError> {
        info!("Requesting next page of User timeline for user #{}", user_id);

        let timeline = self.user_timeline.take().unwrap_or_else(|| {
            let handle = self.core.handle();
            tweet::user_timeline(user_id, true, true, token, &handle).with_page_size(25)
        });

        fn store_tl<'r, 'a>(api: &'r mut RealAPI<'a>, tl: Timeline<'a>) {
            api.user_timeline = Some(tl);
        }
        progress_timeline(self, timeline, store_tl)
    }

    fn likes_timeline_next_page(
        &mut self,
        user_id: u64,
        token: &egg_mode::Token,
    ) -> Result<Vec<Tweet>, APIError> {
        info!("Requesting next page of Likes timeline for user #{}", user_id);

        let timeline = self.likes_timeline.take()
            .unwrap_or_else(|| {
                let handle = self.core.handle();
                tweet::liked_by(user_id, token, &handle).with_page_size(25)
            });

        fn store_tl<'r, 'a>(api: &'r mut RealAPI<'a>, tl: Timeline<'a>) {
            api.likes_timeline = Some(tl);
        }
        progress_timeline(self, timeline, store_tl)
    }
}

fn progress_timeline<'r, 'a, F>(
    api: &'r mut RealAPI<'a>,
    timeline: Timeline<'a>,
    store_tl: F,
) -> Result<Vec<Tweet>, APIError>
where
    F: Fn(&'r mut RealAPI<'a>, Timeline<'a>) -> (),
{
    let future_timeline = timeline.older(None);
    match api.core.run(future_timeline) {
        Ok((new_tl, feed)) => {
            store_tl(api, new_tl);
            Ok(feed.response)
        }
        Err(e) => Err(APIError::TimelineError(e.description().to_string())),
    }
}

#[cfg(test)]
use chrono::prelude::*;
#[cfg(test)]
use egg_mode::tweet::{TweetEntities, TweetSource};
#[cfg(test)]
use std::default::Default;

/// Struct that has a stub implementation of TwitterAPI that doesn't trigger network calls
#[cfg(test)]
#[derive(Debug)]
pub struct TestAPI {
    pub validate_token_answer: Result<(), APIError>,
    pub get_user_id_answer: Result<u64, APIError>,
    pub user_timeline_next_page_answer: Result<Vec<Tweet>, APIError>,
    pub likes_timeline_next_page_answer: Result<Vec<Tweet>, APIError>,
    pub methods_called_in_order: Vec<String>
}

#[cfg(test)]
pub fn sample_tweet(days_ago: i64) -> Tweet {
    let now = Utc::now().timestamp();
    let seconds_past=  days_ago *24 *60 *60;
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
        id: 1,
        in_reply_to_user_id: None,
        in_reply_to_screen_name: None,
        in_reply_to_status_id: None,
        lang: String::from("und"),
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

#[cfg(test)]
impl Default for TestAPI {
    fn default() -> Self {
        TestAPI {
            validate_token_answer: Ok(()),
            get_user_id_answer: Ok(1),
            user_timeline_next_page_answer: Ok(vec![]),
            likes_timeline_next_page_answer: Ok(vec![]),
            methods_called_in_order: Vec::new()
        }
    }
}

#[cfg(test)]
impl TwitterAPI for TestAPI {
    fn validate_token(&mut self, _token: &egg_mode::Token) -> Result<(), APIError> {
        self.methods_called_in_order.push(String::from("validate_token"));
        self.validate_token_answer.clone()
    }

    fn get_user_id(
        &mut self,
        _screen_name: &str,
        _token: &egg_mode::Token,
    ) -> Result<u64, APIError> {
        self.methods_called_in_order.push(String::from("get_user_id"));
        self.get_user_id_answer.clone()
    }

    fn user_timeline_next_page(
        &mut self,
        _user_id: u64,
        _token: &egg_mode::Token,
    ) -> Result<Vec<Tweet>, APIError> {
        self.methods_called_in_order.push(String::from("user_timeline_next_page"));
        self.user_timeline_next_page_answer.clone()
    }

    fn likes_timeline_next_page(
        &mut self,
        _user_id: u64,
        _token: &egg_mode::Token,
    ) -> Result<Vec<Tweet>, APIError> {
        self.methods_called_in_order.push(String::from("likes_timeline_next_page"));
        self.likes_timeline_next_page_answer.clone()
    }
}
