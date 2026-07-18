use std::error::Error;
use log::info;
use crate::audio::source::AudioFrame;
use crate::utils::utils::{to_db_display,
                        exponential_moving_average};
use super::source::DisplaySource;
// use std::sync::mpsc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use log::trace;
use std::time::{Duration, Instant};
use embedded_graphics::{pixelcolor::BinaryColor,prelude::*,primitives::{PrimitiveStyle,Rectangle}};
use embedded_graphics::{mono_font::{ascii::FONT_4X6,MonoTextStyle,},text::{Baseline, Text}};
use linux_embedded_hal::I2cdev;
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::{prelude::*,I2CDisplayInterface,Ssd1306};
use crate::configs::{NUM_BANDS,
    BAND_LABELS,
    I2C_PERIPHERAL_PATH,
    SCREEN_WIDTH,
    GRAPH_HEIGHT,
    UPDATE_INTERVAL_MS};

pub struct OledBars {display: Ssd1306<
                    I2CInterface<I2cdev>,
                    DisplaySize128x64,
                    BufferedGraphicsMode<DisplaySize128x64>
                    >}

impl OledBars {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {

        let i2c = I2cdev::new(I2C_PERIPHERAL_PATH)?;

        let interface = I2CDisplayInterface::new(i2c);

        let mut display = Ssd1306::new(
            interface,
            DisplaySize128x64,
            DisplayRotation::Rotate0,
        )
        .into_buffered_graphics_mode();

        display.init().unwrap();
        display.flush().unwrap();

        Ok(Self { display })
    }
}

impl DisplaySource for OledBars {

    fn self_test(&mut self) -> Result<(), Box<dyn Error>> {
        self.display.clear(BinaryColor::Off).unwrap();

        Rectangle::new(Point::new(0, 0), Size::new(128, 64))
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(&mut self.display).unwrap();

        self.display.flush().unwrap();

        std::thread::sleep(Duration::from_secs(1));

        self.display.clear(BinaryColor::Off).unwrap();
        self.display.flush().unwrap();
        info!("Display initialization health check successful");
        Ok(())
    }
    fn display_results(&mut self,
                       rx_bands: mpsc::Receiver<AudioFrame>,
                       shutdown: Arc<AtomicBool>
    ) {
        let mut last_update = Instant::now();
        let mut band_acc = [0.0; NUM_BANDS];
        let mut count = 0usize;
        let mut accumulated_values = [0.0; NUM_BANDS];

        while !shutdown.load(Ordering::SeqCst) {
        match rx_bands.recv_timeout(Duration::from_millis(100)) {
            Ok(frame) => {
                trace!("Latency Display: {:.3} ms",
                frame.timestamp.elapsed().as_secs_f64() * 1000.0
                );
                //Consume from channel
                for i in 0..NUM_BANDS {
                    band_acc[i] += frame.samples[i];
                }

                count += 1; //Every time count increases it means a new "chunk"
                if is_time_to_update(last_update,count) {
                    self.update_display(&mut band_acc,&mut count, &mut accumulated_values);
                    last_update = Instant::now();
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }
}

impl OledBars {

    fn update_display(&mut self, accumulated_values: &mut [f32],
        count: &mut usize,
        per_band_amplitude: &mut [f32] ){
        compute_display_lvls(accumulated_values, *count, per_band_amplitude);    
        self.draw_screen(per_band_amplitude);
        self.reset_buffer(accumulated_values, count);
    }

    fn draw_screen(&mut self,per_band_amplitude: &[f32]) {
        self.display.clear(BinaryColor::Off).unwrap();
        let bar_width = SCREEN_WIDTH / NUM_BANDS;
        self.draw_bars(per_band_amplitude, bar_width);
        self.draw_labels(bar_width);
        let t = Instant::now();
        self.display.flush().unwrap();
        trace!("Flushing to display = {:.3} ms",t.elapsed().as_secs_f64() * 1000.0);
    }

    fn reset_buffer(&mut self, accumulated_values: &mut [f32], count: &mut usize){
        accumulated_values.fill(0.0);
        *count = 0;    
    }

    fn draw_bars(&mut self, per_band_amplitude: &[f32], bar_width: usize){

        for (i, value) in per_band_amplitude.iter().enumerate() {
            let normalized = (value / 40.0).clamp(0.0, 1.0).powf(0.5);
            let bar_height = (normalized * GRAPH_HEIGHT as f32) as i32;
            let y = GRAPH_HEIGHT as i32 - bar_height;
            let x = (i * bar_width) as i32;
            Rectangle::new(Point::new(x, y),
                            Size::new((bar_width - 2) as u32,bar_height as u32)
                        ).into_styled(PrimitiveStyle::with_fill(BinaryColor::On),
            ).draw(&mut self.display).unwrap();
        }   
    }

    fn draw_labels(&mut self, bar_width: usize){

        let text_style = MonoTextStyle::new(&FONT_4X6,BinaryColor::On);
        
        for (i, label) in BAND_LABELS.iter().enumerate() {
            let x = (i * bar_width + 1) as i32;
            Text::with_baseline(
                label,
                Point::new(x, 63),
                text_style,
                Baseline::Bottom,
            ).draw(&mut self.display)
            .unwrap();
            }
    }
}

fn reset_buffer(accumulated_values: &mut [f32], count: &mut usize){
    accumulated_values.fill(0.0);
    *count = 0;    
}

fn is_time_to_update(last_update: Instant, count:usize)-> bool{
    let is_time: bool = last_update.elapsed() >= Duration::from_millis(UPDATE_INTERVAL_MS) && count > 0;        
    is_time
}

fn compute_display_lvls(accumulated_values: &[f32],
                        count: usize,
                        per_band_amplitude: &mut [f32]){
    for i in 0..NUM_BANDS {
        let avg = to_db_display(accumulated_values[i] / count as f32); // db of average
        per_band_amplitude[i] = exponential_moving_average(per_band_amplitude[i],avg);
    }
}

///////////////////////////////////////////////////////////
/////////////////////TESTING SECTION///////////////////////
///////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn compute_display_levels_from_average() {

        let accumulated = [1.0; NUM_BANDS];
        let mut levels = [0.0; NUM_BANDS];

        compute_display_lvls(
            &accumulated,
            1,
            &mut levels,
        );

        let expected =
            exponential_moving_average(
                0.0,
                to_db_display(1.0),
            );

        for value in levels {
            assert!((value - expected).abs() < 1e-4);
        }
    }

    #[test]
    fn compute_display_levels_preserves_previous_average() {

        let accumulated = [1.0; NUM_BANDS];
        let mut previous = [20.0; NUM_BANDS];

        compute_display_lvls(
            &accumulated,
            1,
            &mut previous,
        );

        let expected =
            exponential_moving_average(
                20.0,
                to_db_display(1.0),
            );

        for value in previous {
            assert!((value - expected).abs() < 1e-4);
        }
    }

    #[test]
    fn reset_buffer_clears_everything() {

        let mut values = [10.0; NUM_BANDS];
        let mut count = 15;

        // this method would need to become a free function or a static helper
        reset_buffer(&mut values, &mut count);

        assert_eq!(count, 0);

        for value in values {
            assert_eq!(value, 0.0);
        }
    }

    #[test]
    fn update_is_false_when_no_frames_arrived() {

        let last = Instant::now();

        assert!(!is_time_to_update(last,0));
    }

    #[test]
    fn update_is_false_before_timeout() {

        let last = Instant::now();

        assert!(!is_time_to_update(last,1));
    }

    #[test]
    fn update_is_true_after_timeout() {

        let last =
            Instant::now()
            - Duration::from_millis(
                UPDATE_INTERVAL_MS + 1
            );

        assert!(is_time_to_update(last,1));
    }
}