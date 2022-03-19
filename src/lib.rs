use hound::WavReader;
use itertools::Itertools;
use rstats::Vecg;
use std::fs::File;
use std::time;
use std::{f32, io::BufReader};
use super_mass::mass;

pub fn find_beginning(
    original: &mut WavReader<BufReader<File>>,
    copy: &mut WavReader<BufReader<File>>,
    offset: u32,
    cut_duration: u32,
) -> u32 {
    let chunk_len = 44100;
    let length = copy.len() - offset;
    let mut chunk_no = (length / 2) + offset;
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

    let beginning = chunk_no;
    beginning
}

fn find_end(
    window_len: time::Duration,
    original: &mut WavReader<BufReader<File>>,
    copy: &mut WavReader<BufReader<File>>,
) -> u32 {
    // Find end cut
    let chunk_len = 44100;
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
    arg_min as u32
}

pub fn find_cut(
    mut original: WavReader<BufReader<File>>,
    mut copy: WavReader<BufReader<File>>,
    window_len: time::Duration,
) -> Vec<[u32; 2]> {
    // let chunk_len = 44100;
    let mut offset = 0;
    let mut cut_duration = 0;
    // let mut length: u32;
    // let mut chunk_no: u32;
    let mut vec_result = Vec::<[u32; 2]>::new();
    while &offset != &copy.len() {
        let chunk_no = find_beginning(&mut original, &mut copy, offset, cut_duration);
        let beginning = chunk_no;

        let arg_min = find_end(window_len, &mut original, &mut copy);
        cut_duration += arg_min as u32;
        offset = chunk_no;
        let end = chunk_no + arg_min as u32;

        // println!(
        //     "orig len: {}, copy + cut: {}",
        //     original.len(),
        //     copy.len() + cut_duration
        // );
        let result = [beginning, end];
        vec_result.push(result);
        if original.len() <= (copy.len() + cut_duration) {
            break;
        }
        original.seek(offset).expect("Seeking error");
    }
    vec_result
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
