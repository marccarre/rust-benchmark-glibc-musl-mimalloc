pub mod channels;
pub mod multithread;
pub use channels::{ChannelConfig, Mpmc, Mpsc, PayloadDist, Spmc};
pub use multithread::{Multithread, MultithreadConfig, SizeDist};
