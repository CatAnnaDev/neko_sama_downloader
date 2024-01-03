use clap::Parser;

#[derive(Parser, Debug)]
#[command(author = "PsykoDev", version, about, long_about = None)]
pub struct Args {
	#[arg(
	short = 's',
	long,
	default_value = "search",
	help = "search or download"
	)]
	pub scan: String,

	#[arg(short = 'u', long)]
	pub url_or_search_word: String,

	#[arg(short = 'l', long, default_value = "vf", help = "vf or vostfr")]
	pub language: String,

	#[arg(short = 't', long, default_value_t = 1)]
	pub thread: u8,
}
