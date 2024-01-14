use std::fmt::{Display, Formatter};
use clap::ArgAction;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author = "PsykoDev", version, about, long_about = None)]
pub struct Args {
    #[arg(
    short = 's',
    long,
    default_value = "",
    help = "add season url to direct download, or type anything to find all correspondent season film etc"
    )]
    pub url_or_search_word: String,

    #[arg(short = 'l', long, default_value = "vf", help = "vf or vostfr")]
    pub language: String,

    #[arg(
    short = 't',
    long,
    default_value_t = 1,
    help = "Thread use to download and process vidéo 1gb/s fiber 20 threads recommended"
    )]
    pub thread: u8,

    #[arg(
    short = 'v',
    long,
    default_value_t = false,
    help = "add more log during process [default: false]",
    action = ArgAction::SetTrue
    )]
    pub debug: bool,

    #[arg(
    short = 'p',
    long = "vlc",
    default_value_t = true,
    help = "create a vlc playlist at the end of process [default: true]",
    action = ArgAction::SetFalse
    )]
    pub vlc_playlist: bool,

    #[arg(
    short = 'i',
    long = "ignore",
    default_value_t = true,
    help = "ignore confirmation to continue if 1 or more episodes is missing to complete the season [default: false]",
    action = ArgAction::SetFalse
    )]
    pub ignore_alert_missing_episode: bool,

    #[arg(
    short = 'm',
    long = "minimized",
    default_value_t = false,
    help = "start chrome minimized or not [default: false]",
    action = ArgAction::SetTrue
    )]
    pub minimized_chrome: bool,
}

impl Display for Args{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Config:\n\
                  Url or Search:\t{}\n\
                  Language:\t{}\n\
                  Threads:\t{}\n\
                  Vlc playlist:\t{}\n\
                  Show Alert:\t{}\n\
                  Minimized:\t{}\n\
                  Debug:\t\t{}",
                  self.url_or_search_word,
                  self.language,
                  self.thread,
                  self.vlc_playlist,
                  self.ignore_alert_missing_episode,
                  self.minimized_chrome,
                  self.debug,
        )
    }
}