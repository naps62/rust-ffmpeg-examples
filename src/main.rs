mod av;
mod cmds;
mod opts;

use clap::Clap;

use cmds::*;
use opts::SubCommand::*;

fn main() {
    let opts = opts::Opts::parse();

    match opts.subcmd {
        Frames(args) => frames::run(args),
        Remux(args) => remux::run(args),
        Transmux(args) => transmux::run(args),
        Transcode(args) => transcode::run(args),
        Formats => formats::run(),
    }
}
