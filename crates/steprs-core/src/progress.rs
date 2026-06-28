/// Progress callback for large-file WASM / CLI parsing.
pub trait ParseProgress: Send + Sync {
    fn on_phase(&self, phase: ParsePhase, done: usize, total: Option<usize>);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParsePhase {
    Header,
    DataScan,
    Indexing,
    Topology,
    Features,
    Toolpath,
    PostProcess,
    StockSim,
}

#[derive(Debug, Default)]
pub struct NoopProgress;

impl ParseProgress for NoopProgress {
    fn on_phase(&self, _phase: ParsePhase, _done: usize, _total: Option<usize>) {}
}
