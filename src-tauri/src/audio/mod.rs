pub mod capture;
pub mod device;
pub mod playback;
pub mod resample;
pub mod vad;

pub use device::{list_devices, AudioDevice};
pub use playback::play_sound;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioState {
    Idle,
    Starting,
    Recording,
    Processing,
}
