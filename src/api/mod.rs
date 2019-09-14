use crate::EnvValues;
use egg_mode;
use egg_mode::tweet;
use egg_mode::tweet::{Timeline, Tweet};
use std::error::Error;
use std::fmt;
use tokio::runtime::current_thread::block_on_all;

/// Defines errors that can happen when calling the API methods
#[derive(Debug, Clone, PartialEq)]
pub enum APIError {
    InvalidToken,
    TimelineError(String),
    UserDetailsError(String),
    ErasureError(String),
}

impl fmt::Display for APIError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            APIError::InvalidToken => write!(
                f,
                "Invalid or Expired Token used in the request to Twitter API"
            ),
            APIError::TimelineError(s) => {
                write!(f, "Error when retrieving data from a timeline: {}", s)
            }
            APIError::UserDetailsError(s) => write!(f, "Failure obtaining user details: {}", s),
            APIError::ErasureError(s) => {
                write!(f, "Failure removing link between tweet and user: {}", s)
            }
        }
    }
}

/// Trait that defines methods we need to interact with the Twitter API
/// Created so we can avoid real API calls during testing, using a stub instead
pub trait TwitterAPI {
    /// Returns the next page available of user timeline for given user id, which contains tweets published (or retweeted) by user
    fn user_timeline_next_page(&mut self) -> Result<Vec<Tweet>, APIError>;

    /// Returns the next page available of tweets liked by given user id
    fn likes_timeline_next_page(&mut self) -> Result<Vec<Tweet>, APIError>;

    /// Unlikes a tweet the user liked before
    fn unlike_tweet(&mut self, tweet: &Tweet) -> Result<(), APIError>;

    // Unretweets a tweets the user retweeted before
    fn unretweet_tweet(&mut self, tweet: &Tweet) -> Result<(), APIError>;

    // Erases a tweet posted by the user
    fn erase_tweet(&mut self, tweet: &Tweet) -> Result<(), APIError>;
}

/// Struct that has an implementation of TwitterAPI that calls twitter servers
pub struct RealAPI<'a> {
    pub user_id: u64,
    pub token: egg_mode::Token,
    pub user_timeline: Option<Timeline<'a>>,
    pub likes_timeline: Option<Timeline<'a>>,
}

impl<'a> RealAPI<'a> {
    /// Uses a set of environment variables to initialise an instance to Twitter API
    ///
    /// # Side Effects
    ///
    /// Does calls to Twitter API for token validation
    ///
    /// # Error scenarios
    ///
    /// The method will return an `Err` if:
    ///
    /// - the values in `EnvValues` aren't valid tokens to interact with the API
    /// - the `api` parameter returns some error when we use its methods
    ///
    pub fn new(env: EnvValues) -> Result<RealAPI<'a>, APIError> {
        info!("Creating Real API object");

        let con_token = egg_mode::KeyPair::new(env.consumer_key, env.consumer_secret);
        let access_token = egg_mode::KeyPair::new(env.access_key, env.access_secret);
        let token = egg_mode::Token::Access {
            consumer: con_token,
            access: access_token,
        };

        let mut api = RealAPI {
            user_id: 0,
            token,
            user_timeline: None,
            likes_timeline: None,
        };

        RealAPI::validate_token(&mut api)?;
        RealAPI::obtain_user_id(&mut api, &env.user_handle)?;

        info!("Welcome back, {}!", &env.user_handle);

        Ok(api)
    }

    fn validate_token(api: &mut RealAPI) -> Result<(), APIError> {
        info!("Verifying validity of Token by querying Twitter API");

        if let Err(err) = block_on_all(egg_mode::verify_tokens(&api.token)) {
            error!("We've hit an error using your tokens: {:?}. Invalid tokens, the application can't continue.", err);
            Err(APIError::InvalidToken)
        } else {
            info!("Tokens seem to be valid");
            Ok(())
        }
    }

    fn obtain_user_id(api: &mut RealAPI, screen_name: &str) -> Result<(), APIError> {
        info!("Requesting user id for user {}", screen_name);

        let query_for_user = block_on_all(egg_mode::user::show(screen_name, &api.token));

        let user_info = match query_for_user {
            Ok(uinfo) => uinfo,
            Err(e) => return Err(APIError::UserDetailsError(e.description().to_string())),
        };

        info!(
            "Retrieved user id {} for {} (@{})",
            user_info.id, user_info.name, user_info.screen_name
        );

        api.user_id = user_info.id;

        Ok(())
    }
}

impl<'a> TwitterAPI for RealAPI<'a> {
    fn user_timeline_next_page(&mut self) -> Result<Vec<Tweet>, APIError> {
        info!(
            "Requesting next page of User timeline for user #{}",
            self.user_id
        );

        let timeline = self.user_timeline.take().unwrap_or_else(|| {
            tweet::user_timeline(self.user_id, true, true, &self.token).with_page_size(25)
        });

        fn store_tl<'r, 'a>(api: &'r mut RealAPI<'a>, tl: Timeline<'a>) {
            api.user_timeline = Some(tl);
        }
        progress_timeline(self, timeline, store_tl)
    }

    fn likes_timeline_next_page(&mut self) -> Result<Vec<Tweet>, APIError> {
        info!(
            "Requesting next page of Likes timeline for user #{}",
            self.user_id
        );

        let timeline = self.likes_timeline.take().unwrap_or_else(|| {
            tweet::liked_by(self.user_id, &self.token).with_page_size(25)
        });

        fn store_tl<'r, 'a>(api: &'r mut RealAPI<'a>, tl: Timeline<'a>) {
            api.likes_timeline = Some(tl);
        }
        progress_timeline(self, timeline, store_tl)
    }

    fn unlike_tweet(&mut self, tweet: &Tweet) -> Result<(), APIError> {
        if tweet.favorited.unwrap_or(false) {
            info!(
                "Requesting unlike of tweet #{} posted at {}",
                tweet.id, tweet.created_at
            );

            block_on_all(tweet::unlike(tweet.id, &self.token))
                .map_err(|e| APIError::ErasureError(e.description().to_string()))
                .map(|_| ())
        } else {
            warn!(
                "Tried to unlike tweet #{} which it not favourited by the user",
                tweet.id
            );
            Ok(())
        }
    }

    fn unretweet_tweet(&mut self, tweet: &Tweet) -> Result<(), APIError> {
        if tweet.retweeted.unwrap_or(false) {
            info!(
                "Requesting unretweet of tweet #{} posted at {}",
                tweet.id, tweet.created_at
            );

            block_on_all(tweet::unretweet(tweet.id, &self.token))
                .map_err(|e| APIError::ErasureError(e.description().to_string()))
                .map(|_| ())
        } else {
            warn!(
                "Tried to unretweet tweet #{} which it not retweeted by the user",
                tweet.id
            );
            Ok(())
        }
    }

    fn erase_tweet(&mut self, tweet: &Tweet) -> Result<(), APIError> {
        let is_own_tweet = match tweet.user {
            Some(ref tu) if tu.id != self.user_id => false,
            _ => true,
        };

        if is_own_tweet {
            info!(
                "Requesting removal of tweet #{} posted at {}",
                tweet.id, tweet.created_at
            );

            if let Err(e) = block_on_all(tweet::delete(tweet.id, &self.token)) {
                warn!("Couldn't erase tweet #{}. Error received: {}", tweet.id, e);
            }
        } else {
            warn!(
                "Tried to delete tweet #{} which it not posted by the user",
                tweet.id
            );
        }

        Ok(())
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
    match block_on_all(future_timeline) {
        Ok((new_tl, feed)) => {
            store_tl(api, new_tl);
            Ok(feed.response)
        }
        Err(e) => Err(APIError::TimelineError(e.description().to_string())),
    }
}

#[cfg(test)]
use std::default::Default;

/// Struct that has a stub implementation of TwitterAPI that doesn't trigger network calls
#[cfg(test)]
#[derive(Debug)]
pub struct TestAPI {
    pub user_timeline_next_page_answer: Result<Vec<Tweet>, APIError>,
    pub likes_timeline_next_page_answer: Result<Vec<Tweet>, APIError>,
    pub unlike_tweet_answer: Result<(), APIError>,
    pub unretweet_tweet_answer: Result<(), APIError>,
    pub erase_tweet_answer: Result<(), APIError>,
    pub methods_called_in_order: Vec<String>,
}

#[cfg(test)]
impl Default for TestAPI {
    fn default() -> Self {
        TestAPI {
            user_timeline_next_page_answer: Ok(vec![]),
            likes_timeline_next_page_answer: Ok(vec![]),
            unlike_tweet_answer: Ok(()),
            unretweet_tweet_answer: Ok(()),
            erase_tweet_answer: Ok(()),
            methods_called_in_order: Vec::new(),
        }
    }
}

#[cfg(test)]
impl TwitterAPI for TestAPI {
    fn user_timeline_next_page(&mut self) -> Result<Vec<Tweet>, APIError> {
        self.methods_called_in_order
            .push(String::from("user_timeline_next_page"));
        self.user_timeline_next_page_answer.clone()
    }

    fn likes_timeline_next_page(&mut self) -> Result<Vec<Tweet>, APIError> {
        self.methods_called_in_order
            .push(String::from("likes_timeline_next_page"));
        self.likes_timeline_next_page_answer.clone()
    }

    fn unlike_tweet(&mut self, _tweet: &Tweet) -> Result<(), APIError> {
        self.methods_called_in_order
            .push(String::from("unlike_tweet"));
        self.unlike_tweet_answer.clone()
    }

    fn unretweet_tweet(&mut self, _tweet: &Tweet) -> Result<(), APIError> {
        self.methods_called_in_order
            .push(String::from("unretweet_tweet"));
        self.unretweet_tweet_answer.clone()
    }

    fn erase_tweet(&mut self, _tweet: &Tweet) -> Result<(), APIError> {
        self.methods_called_in_order
            .push(String::from("erase_tweet"));
        self.erase_tweet_answer.clone()
    }
}
