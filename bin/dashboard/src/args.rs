#[derive(clap::Parser, Clone, Debug)]
#[command(version, about, long_about=None)]
pub struct Args {
    #[arg(
        short,
        long,
        default_value = "127.0.0.1:5000",
        help = "HTTP listen address"
    )]
    pub listen: String,
    #[arg(
        short,
        long,
        default_value = "sqlite://data.db?mode=rwc",
        help = "Database connection string"
    )]
    pub database: String,
    #[arg(
        short,
        long,
        help = "Authorize token signature key (default: random key)"
    )]
    pub secret: Option<String>,
}
