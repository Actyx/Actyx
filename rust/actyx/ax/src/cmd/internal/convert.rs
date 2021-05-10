use crate::cmd::AxCliCommand;
use futures::{prelude::*, stream, Stream};
use structopt::StructOpt;
use swarm::convert::{convert_from_v1, ConversionOptions};
use util::formats::{ActyxOSResult, ActyxOSResultExt};

#[derive(StructOpt, Debug)]
pub struct ConvertFromV1Opts {
    #[structopt(help("path to the source index store. The name of the source block store will be derived from this by appending '-blocks.sqlite'."))]
    source: String,

    #[structopt(help("path to the target index store. The name of the target block store will be derived from this by appending '-blocks.sqlite'."))]
    target: String,

    #[structopt(long, help("do not run gc after conversion"))]
    no_gc: bool,

    #[structopt(long, help("do not run vacuum after conversion"))]
    no_vacuum: bool,
}

pub struct ConvertFromV1;
impl AxCliCommand for ConvertFromV1 {
    type Opt = ConvertFromV1Opts;
    type Output = String;
    fn run(opts: ConvertFromV1Opts) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let conversion_options = ConversionOptions {
            gc: !opts.no_gc,
            vacuum: !opts.no_vacuum,
        };
        let result = convert_from_v1(&opts.source, &opts.target, conversion_options)
            .map(|_| format!("Conversion done. Target db at {}", opts.target))
            .ax_err_ctx(util::formats::ActyxOSCode::ERR_NODE_UNREACHABLE, "Convert failed");
        Box::new(stream::once(future::ready(result)))
    }

    fn pretty(result: Self::Output) -> String {
        result
    }
}
