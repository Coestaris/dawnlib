#[cfg(test)]
mod tests {
    extern crate test;
    use crate::dsp::detect_features;
    use crate::sample::{InterleavedBlock, PlanarBlock};
    use std::panic;
    use test::Bencher;
    use crate::{BLOCK_SIZE, CHANNELS_COUNT};

    #[test]
    fn copy_from_planar_vec_full_test() {
        let mut block = PlanarBlock::<f32>::default();
        let input: [Vec<f32>; CHANNELS_COUNT] = [
            (0..BLOCK_SIZE).map(|i| i as f32).collect(),
            (0..BLOCK_SIZE).map(|i| i as f32 + 1.0).collect(),
        ];

        block.copy_from_planar_vec(&input, 0, BLOCK_SIZE);

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                assert_eq!(block.samples[channel][i], input[channel][i]);
            }
        }
    }

    #[test]
    fn copy_from_planar_vec_fail_if_not_end_of_block() {
        let result = panic::catch_unwind(|| {
            let mut block = PlanarBlock::<f32>::default();
            let input: [Vec<f32>; CHANNELS_COUNT] = [
                (0..BLOCK_SIZE).map(|i| i as f32).collect(),
                (0..BLOCK_SIZE).map(|i| i as f32 + 1.0).collect(),
            ];

            // Panic if we try to copy more than the block size
            block.copy_from_planar_vec(&input, 0, BLOCK_SIZE + 1);
        });

        assert!(
            result.is_err(),
            "Expected panic when copying more than block size"
        );
    }

    #[test]
    fn copy_from_test() {
        let mut block1 = PlanarBlock::<f32>::default();
        let mut block2 = PlanarBlock::<f32>::default();

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                block1.samples[channel][i] = i as f32 + channel as f32;
            }
        }

        block2.copy_from(&block1);

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                assert_eq!(block2.samples[channel][i], i as f32 + channel as f32);
            }
        }
    }

    #[test]
    fn copy_into_interleaved_test() {
        detect_features();

        let mut block = PlanarBlock::<f32>::default();
        let mut interleaved = InterleavedBlock::<f32>::default();

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                block.samples[channel][i] = i as f32 + channel as f32 * 100.0;
            }
        }

        block.copy_into_interleaved(&mut interleaved);

        for i in 0..BLOCK_SIZE {
            for channel in 0..CHANNELS_COUNT {
                let expected = i as f32 + channel as f32 * 100.0;
                let actual = interleaved.samples[i].channels[channel];
                assert_eq!(
                    actual, expected,
                    "Mismatch at sample {}, channel {}",
                    i, channel
                );
            }
        }
    }

    #[bench]
    fn copy_into_interleaved_bench(b: &mut Bencher) {
        detect_features();

        let mut planar = PlanarBlock::<f32>::default();
        let mut interleaved: InterleavedBlock<f32> = InterleavedBlock::default();

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                planar.samples[channel][i] = i as f32 + channel as f32;
            }
        }

        b.iter(|| {
            planar.copy_into_interleaved(&mut interleaved);
        });
    }

    #[test]
    fn add_test() {
        detect_features();

        let mut block1 = PlanarBlock::<f32>::default();
        let mut block2 = PlanarBlock::<f32>::default();

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                block1.samples[channel][i] = i as f32 + channel as f32;
                block2.samples[channel][i] = (i as f32 + channel as f32) * 2.0;
            }
        }

        block1.add(&block2);

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                assert_eq!(
                    block1.samples[channel][i],
                    (i as f32 + channel as f32) * 3.0
                );
            }
        }
    }

    #[bench]
    fn add_bench(b: &mut Bencher) {
        detect_features();

        let mut block1 = PlanarBlock::<f32>::default();
        let mut block2 = PlanarBlock::<f32>::default();

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                block1.samples[channel][i] = i as f32 + channel as f32;
                block2.samples[channel][i] = (i as f32 + channel as f32) * 2.0;
            }
        }

        b.iter(|| {
            block1.add(&block2);
        });
    }

    #[test]
    fn addm_test() {
        detect_features();

        let mut block1 = PlanarBlock::<f32>::default();
        let mut block2 = PlanarBlock::<f32>::default();

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                block1.samples[channel][i] = i as f32 + channel as f32;
                block2.samples[channel][i] = (i as f32 + channel as f32) * 3.0;
            }
        }

        block1.addm(&block2, 0.5);
        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                assert_eq!(
                    block1.samples[channel][i],
                    (i as f32 + channel as f32) + (i as f32 + channel as f32) * 3.0 * 0.5
                );
            }
        }
    }
}
