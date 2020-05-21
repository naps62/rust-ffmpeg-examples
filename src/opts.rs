extern crate clap;

use clap::Clap;

#[derive(Clap, Debug)]
#[clap(version = "1.0", author = "Miguel Palhas <mpalhas@gmail.com")]
pub struct Opts {
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Clap, Debug)]
pub enum SubCommand {
    Frames(Frames),
}

#[derive(Clap, Debug)]
pub struct Frames {
    #[clap(short = "i", long = "input")]
    pub input: String,
}
