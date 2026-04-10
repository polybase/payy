use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct FetchArgs {
    pub url: String,

    #[arg(short = 'X', long)]
    pub method: Option<String>,

    #[arg(short = 'H', long = "header")]
    pub headers: Vec<String>,

    #[arg(short = 'd', long, conflicts_with = "data_file")]
    pub data: Option<String>,

    #[arg(long, conflicts_with = "data")]
    pub data_file: Option<String>,

    #[arg(id = "fetch-output", short = 'o', long = "output")]
    pub output_path: Option<String>,

    #[arg(short = 'v', long, default_value_t = false)]
    pub verbose: bool,

    #[arg(short = 'L', long, default_value_t = false)]
    pub follow_redirects: bool,

    #[arg(long, default_value_t = 10)]
    pub max_redirects: usize,

    #[arg(long)]
    pub connect_timeout: Option<u64>,

    #[arg(long)]
    pub timeout: Option<u64>,

    #[arg(long)]
    pub max_fee: Option<String>,

    #[arg(long, value_delimiter = ',', value_name = "NAME|ID")]
    pub allowed_chains: Vec<String>,

    #[arg(long, default_value_t = false)]
    pub no_pay: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Allow loopback HTTP payment challenges for development and fixtures"
    )]
    pub dev: bool,
}
