mod cli;
mod filter;
mod functions;
mod git;
mod hotspots;
mod imports;
mod jsonc;
mod lang;
mod lines;
mod output;
mod parse;
mod progress;
mod resolve;
mod scan;
mod tree;

fn main() {
    std::process::exit(cli::run());
}
