use clap::ArgAction;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author = "PsykoDev", version, about, long_about = None)]
pub struct Args {
    #[arg(short = 's', long, help = "add season url to direct download, or type anything to find all correspondent season film etc")]
    pub url_or_search_word: String,

    #[arg(short = 'l', long, default_value = "vf", help = "vf or vostfr")]
    pub language: String,

    #[arg(short = 't', long, default_value_t = 1, help = "Default 1")]
    pub thread: u8,

    #[arg(
        short = 'v',
        long,
        default_value_t = false,
        help = "add more log during process [default: false]",
        action=ArgAction::SetTrue
    )]
    pub debug: bool,

    #[arg(
        short = 'p',
        long = "vlc",
        default_value_t = true,
        help = "create a vlc playlist at the end of process [default: true]",
        action=ArgAction::SetFalse
    )]
    pub vlc_playlist: bool,
}
