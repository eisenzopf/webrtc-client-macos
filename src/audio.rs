use cpal::traits::{DeviceTrait, StreamTrait, HostTrait};
use std::sync::Arc;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_remote::TrackRemote;

pub struct AudioDevice {
    input_device: cpal::Device,
    output_device: cpal::Device,
}

impl AudioDevice {
    pub fn new() -> anyhow::Result<Arc<Self>> {
        let host = cpal::default_host();
        let input_device = host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device found"))?;
        let output_device = host.default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No output device found"))?;

        Ok(Arc::new(Self {
            input_device,
            output_device,
        }))
    }

    pub async fn capture_local_audio(
        &self,
        track: Arc<TrackLocalStaticRTP>,
    ) -> anyhow::Result<()> {
        let config = self.input_device.default_input_config()?;
        let track = track.clone();
        
        let stream = self.input_device.build_input_stream(
            &config.config(),
            move |_data: &[f32], _: &cpal::InputCallbackInfo| {
                // Convert audio samples to opus packets
                // This is a simplified version - you'll need to implement proper opus encoding
                let _track = track.clone();
                // TODO: Implement audio encoding and sending
            },
            |err| eprintln!("Error in input stream: {}", err),
            None,
        )?;
        
        stream.play()?;
        Ok(())
    }

    pub async fn play_remote_track(
        &self,
        track: Arc<TrackRemote>,
    ) -> anyhow::Result<()> {
        let config = self.output_device.default_output_config()?;
        let track = track.clone();
        
        let stream = self.output_device.build_output_stream(
            &config.config(),
            move |_data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let _track = track.clone();
                // TODO: Implement audio decoding and playback
            },
            |err| eprintln!("Error in output stream: {}", err),
            None,
        )?;
        
        stream.play()?;
        Ok(())
    }
}