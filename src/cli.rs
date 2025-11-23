use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct Args {
    pub db_path: String,

    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, Subcommand, Clone)]
pub enum Cmd {
    #[clap(name = ".dbinfo")]
    DatabaseInfo,
    #[clap(name = ".tables")]
    Tables,
    #[clap(name = "sql")]
    Sql { query: String },
}
