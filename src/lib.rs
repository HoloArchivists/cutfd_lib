use hound::WavReader;
use itertools::Itertools;
use rstats::Vecg;
use std::fs::File;
use std::time;
use std::{f32, io::BufReader};
use super_mass::mass;

fn format_time(time: f32) -> String {
    let milliseconds = (time % 1.0 * 1000.0) as usize;
    let tot_seconds = time as usize;
    let seconds = tot_seconds % 60;
    let minutes = (tot_seconds / 60) % 60;
    let hours = tot_seconds / 60 / 60;
    format!("{}:{}:{}.{}", hours, minutes, seconds, milliseconds)
}

pub fn find_cut(
    mut original: WavReader<BufReader<File>>,
    mut copy: WavReader<BufReader<File>>,
    window_len: time::Duration,
) {
    let chunk_len = 44100;
    let mut offset = 0;
    let mut cut_duration = 0;
    let mut length: u32;
    let mut chunk_no: u32;
    while offset != copy.len() {
        length = copy.len() - offset;
        chunk_no = (length / 2) + offset;
        // Find start cut
        let mut stage = 1;
        // println!(
        //     "orig length: {} , offset: {} , cut_duration: {}",
        //     copy.len(),
        //     offset,
        //     cut_duration
        // );
        // println!(
        //     "chunk_no wo offset: {} , length wo offset: {}",
        //     chunk_no - offset,
        //     length
        // );
        for _ in 0..((length as f32).log2() as usize) {
            // println!("chunk_no: {}", chunk_no);
            original
                .seek(chunk_no + cut_duration)
                .expect("seeking error");
            copy.seek(chunk_no).expect("seeking error");
            let orig_sampl: Vec<f32> = original
                .samples::<f32>()
                .map(|x| x.expect("Failed to read sample"))
                .chunks(chunk_len as usize)
                .into_iter()
                .next()
                .expect("Failed to fetch sample")
                .collect();

            let cp_sampl: Vec<f32> = copy
                .samples::<f32>()
                .map(|x| x.expect("Failed to read sample"))
                .chunks(chunk_len as usize)
                .into_iter()
                .next()
                .expect("Failed to fetch sample")
                .collect();
            let corr = orig_sampl.mediancorr(&cp_sampl);
            // println!("chunk_no: {}, corr: {}", chunk_no + offset_cut, corr);
            if corr > 0.95 {
                chunk_no = chunk_no + length / (2 as u32).pow(stage)
            } else {
                chunk_no = chunk_no - length / (2 as u32).pow(stage)
            }
            stage += 1;
        }

        let tot_seconds = chunk_no as f32 / 44100.0;
        let time_str = format_time(tot_seconds);
        println!("Cut at {}", time_str);

        // Find end cut
        let window_len = window_len.as_secs() * 44100;
        let mut window_samples: Vec<f64> = Vec::new();
        let mut orig_samples = original
            .samples::<f32>()
            .map(|x| x.expect("Failed to read sample") as f64);
        let cp_sampl: Vec<f64> = copy
            .samples::<f32>()
            .map(|x| x.expect("Failed to read sample") as f64)
            .chunks(chunk_len as usize)
            .into_iter()
            .next()
            .unwrap()
            .collect();
        for _ in 0..window_len {
            window_samples.extend(orig_samples.next())
        }
        let distances = mass(&window_samples, &cp_sampl);
        let (arg_min, _) = distances
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).expect("Encountered Not a Number(NaN)"))
            .unwrap();
        cut_duration += arg_min as u32;
        offset = chunk_no;
        let tot_seconds = (chunk_no + arg_min as u32) as f32 / 44100.0;
        let time_str = format_time(tot_seconds);
        println!("Cut end at {}", time_str);
        // println!(
        //     "orig len: {}, copy + cut: {}",
        //     original.len(),
        //     copy.len() + cut_duration
        // );
        if original.len() <= (copy.len() + cut_duration) {
            break;
        }
        original.seek(offset).expect("Seeking error");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
