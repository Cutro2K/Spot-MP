#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaybackFlow {
    Normal,
    Random,
    InCycle,
}
