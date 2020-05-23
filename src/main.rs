mod av;
mod cmds;
mod opts;

use clap::Clap;

fn main() {
    let opts = opts::Opts::parse();

    match opts.subcmd {
        opts::SubCommand::Frames(args) => {
            cmds::frames::run(args);
        }
        opts::SubCommand::Remux(args) => {
            cmds::remux::run(args);
        }
    }
}
