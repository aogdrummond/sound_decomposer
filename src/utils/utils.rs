

pub fn to_db_display(
    amplitude: f32
) -> f32 {

    let db = 20.0 * amplitude.max(1e-10).log10();
    db.clamp(-80.0, 0.0) + 80.0
}
// Low-pass filter for smoothing noisy signals
pub fn exponential_moving_average(
    current_value: f32,
    average: f32
) -> f32 {
    let ema: f32 = 0.8 * current_value + 0.2 * average;
    ema
}

///////////////////////////////////////////////////////////
/////////////////////TESTING SECTION///////////////////////
///////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {

    use super::*;

    const EPSILON: f32 = 1e-4;

    //
    // to_db_display()
    //

    #[test]
    fn db_of_unity_is_maximum_display() {
        assert!((to_db_display(1.0) - 80.0).abs() < EPSILON);
    }

    #[test]
    fn db_of_tenth_is_sixty() {
        // 20*log10(0.1) = -20 dB
        // Display value = 60
        assert!((to_db_display(0.1) - 60.0).abs() < EPSILON);
    }

    #[test]
    fn db_of_hundredth_is_forty() {
        // -40 dB
        assert!((to_db_display(0.01) - 40.0).abs() < EPSILON);
    }

    #[test]
    fn db_floor_is_zero() {
        // Anything below -80 dB should clamp to zero
        assert_eq!(to_db_display(0.0), 0.0);
    }

    #[test]
    fn negative_amplitude_is_treated_as_zero() {
        assert_eq!(to_db_display(-1.0), 0.0);
    }

    #[test]
    fn tiny_amplitude_is_clamped() {
        assert_eq!(to_db_display(1e-20), 0.0);
    }

    #[test]
    fn values_above_unity_are_clamped_to_maximum() {
        assert_eq!(to_db_display(2.0), 80.0);
        assert_eq!(to_db_display(100.0), 80.0);
    }

    #[test]
    fn output_is_always_between_zero_and_eighty() {

        let test_values = [
            -10.0,
            0.0,
            1e-20,
            1e-6,
            0.01,
            0.1,
            1.0,
            2.0,
            1000.0,
        ];

        for value in test_values {
            let db = to_db_display(value);

            assert!(
                (0.0..=80.0).contains(&db),
                "Input {value} produced invalid output {db}"
            );
        }
    }

    //
    // exponential_moving_average()
    //

    #[test]
    fn ema_of_identical_values_is_the_same_value() {
        assert!((exponential_moving_average(10.0, 10.0) - 10.0).abs() < EPSILON);
    }

    #[test]
    fn ema_weights_current_sample_correctly() {

        let ema = exponential_moving_average(10.0, 0.0);

        assert!((ema - 8.0).abs() < EPSILON);
    }

    #[test]
    fn ema_weights_previous_average_correctly() {

        let ema = exponential_moving_average(0.0, 10.0);

        assert!((ema - 2.0).abs() < EPSILON);
    }

    #[test]
    fn ema_handles_negative_values() {

        let ema = exponential_moving_average(-10.0, 10.0);

        assert!((ema + 6.0).abs() < EPSILON);
    }

    #[test]
    fn ema_result_is_between_inputs() {

        let cases = [
            (0.0, 10.0),
            (10.0, 0.0),
            (-10.0, 10.0),
            (5.0, 20.0),
            (-5.0, -20.0),
        ];

        for (current, average) in cases {

            let ema = exponential_moving_average(current, average);

            let min = current.min(average);
            let max = current.max(average);

            assert!(
                ema >= min && ema <= max,
                "EMA {} not between {} and {}",
                ema,
                min,
                max
            );
        }
    }

    #[test]
    fn ema_is_finite_for_large_values() {

        let ema = exponential_moving_average(1e20, -1e20);

        assert!(ema.is_finite());
    }
}