# Twitter Privacy

A tool to erase old tweets from your timeline. It will find all tweets older than a certain amount of days and it will:

- erase tweets you published
- undo retweets
- undo favourite/likes

Note that unfortunately some old tweets are not accessible via the API, so you can't get rid of them.

Written in Rust as a project to learn more about the language and associated tooling. Beware code quality :) Feedback always welcome.

## Usage

You can run with `cargo run` as usual. 
It is recommended to build the binary (`cargo build --release`) and use that binary ina  cron job that runs regularly.

## Configuration

The application requires a set of environment variables to be set-up:

```bash
export TP_CONSUMER_KEY="consumer_key"
export TP_CONSUMER_SECRET="consumer_secret"
export TP_ACCESS_KEY="access_key"
export TP_ACCESS_SECRET="access_secret"
export TP_USER_HANDLE="yourHandle"
export TP_PRESERVE_DAYS=60
```

You can use an `.env` file to define the values. The file must be at the same location you runt he executable from. Otherwise, just set up the environemnt variables.

# Contribution policy

Contributions via GitHub pull requests are gladly accepted from their original author. Along with any pull requests, please state that the contribution is your original work and that you license the work to the project under the project's open source license. Whether or not you state this explicitly, by submitting any copyrighted material via pull request, email, or other means you agree to license the material under the project's open source license and warrant that you have the legal authority to do so.

# License

This code is open source software licensed under the Apache-2.0 license.

# Motivation behind the project

Privacy is a tricky subject and can't be addressed in a Readme file. At the moment of this writing social media seems to be a liability for users. On one hand, social media *can* be useful as a way to obtain information from selected channels, tailored to your needs or tastes. On the other, interactions with people in social media *can* be hard to navigate correctly. 

It seems in some scenarios tehse interactions have had a direct impact on the employability of people. I'm not talking about some famous person exposing views that go against   basic tenets of the Human Rights Declaration; I refer to more mundane, albeit confrontational, interactions between people that have had lasting consequences in the real world.

I acknowledge this is a form of self-censorship that shouldn't be required in a free society. You can call me a coward. But, as thing stands and until we get better at social media it seems silly not to mitigate risks like these.
