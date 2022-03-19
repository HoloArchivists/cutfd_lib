use hound::WavReader;
use rstats::Vecg;
use std::fs::File;
use std::time;
use std::{f32, io::BufReader};
use super_mass::mass;

/// Finds the beginning of a cut between two wav files.
/// cut_duration is the length of the cut already taken into account
/// offset is the length on both file that has been taken into account
/// Example:
/// File_1 is the original and has a length of 100.
/// File_2 is the copy and has 2 cuts at 23, 32, and 55 of length 10, 4, and 15 respectively.
/// Running the function with offset and cut_duration of 0 will return 23
/// If you want to find to second cut, however, you'll have to use an offset of 23 and cut_duration of 10.
/// The function will then return 32. For the next, call the values will have to be 32 and 14 (10+4), etc.
///
/// Returns the sample number.
///
/// # Examples
///
/// ```
/// let mut orig_reader = hound::WavReader::open("./tests/single_cut.wav")
///     .expect("Failed to open input waveform");
/// assert_eq!(orig_reader.spec().channels, 1);
/// assert_eq!(orig_reader.spec().sample_rate, 44100);
/// assert_eq!(orig_reader.spec().bits_per_sample, 32);

/// let mut copy_reader = hound::WavReader::open("./tests/single_cut_copy.wav")
///     .expect("Failed to open input waveform");
/// assert_eq!(copy_reader.spec().channels, 1);
/// assert_eq!(copy_reader.spec().sample_rate, 44100);
/// assert_eq!(copy_reader.spec().bits_per_sample, 32);

// let beginning = find_beginning(&mut orig_reader, &mut copy_reader, 0, 0);
// assert_eq!(beginning, 191443972);
/// ```
pub fn find_beginning(
    original: &mut WavReader<BufReader<File>>,
    copy: &mut WavReader<BufReader<File>>,
    offset: u32,
    cut_duration: u32,
) -> Result<u32, hound::Error> {
    let chunk_len = 44100;
    let length = copy.len() - offset;
    let mut chunk_no = (length / 2) + offset;
    let mut stage = 1;
    for _ in 0..((length as f32).log2() as usize) {
        // println!("chunk_no: {}", chunk_no);
        original
            .seek(chunk_no + cut_duration)
            .expect("seeking error");
        copy.seek(chunk_no).expect("seeking error");

        let mut orig_sampl = original.samples::<f32>();
        let mut orig_sampl_chunk = Vec::<f32>::new();
        for _ in 0..chunk_len {
            match orig_sampl.next() {
                Some(sample) => match sample {
                    Ok(sample) => Ok(orig_sampl_chunk.push(sample)),
                    Err(err) => Err(err),
                },
                None => Err(hound::Error::InvalidSampleFormat),
            };
        }

        let mut cp_sampl = copy.samples::<f32>();
        let mut cp_sampl_chunk = Vec::<f32>::new();
        for _ in 0..chunk_len {
            match cp_sampl.next() {
                Some(sample) => match sample {
                    Ok(sample) => Ok(cp_sampl_chunk.push(sample)),
                    Err(err) => Err(err),
                },
                None => Err(hound::Error::InvalidSampleFormat),
            };
        }
        let corr = orig_sampl_chunk.correlation(&cp_sampl_chunk);
        // println!("{}", i);
        // println!("chunk_no: {}, corr: {}", chunk_no + cut_duration, corr);
        if corr > 0.95 {
            chunk_no = chunk_no + length / (2 as u32).pow(stage)
        } else {
            chunk_no = chunk_no - length / (2 as u32).pow(stage)
        }
        stage += 1;
    }

    let beginning = chunk_no;
    Ok(beginning)
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
    let mut orig_samples = original.samples::<f32>();
    let mut cp_sampl = copy.samples::<f32>();
    let mut cp_sampl_chunk: Vec<f64> = Vec::new();
    for _ in 0..chunk_len {
        match cp_sampl.next() {
            Some(sample) => match sample {
                Ok(sample) => Ok(cp_sampl_chunk.push(sample as f64)),
                Err(err) => Err(err),
            },
            None => Err(hound::Error::InvalidSampleFormat),
        };
    }
    for _ in 0..window_len {
        match orig_samples.next() {
            Some(sample) => match sample {
                Ok(sample) => Ok(window_samples.push(sample as f64)),
                Err(err) => Err(err),
            },
            None => Err(hound::Error::InvalidSampleFormat),
        };
    }

    let distances = mass(&window_samples, &cp_sampl_chunk);
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
) -> Result<Vec<[u32; 2]>, hound::Error> {
    // let chunk_len = 44100;
    let mut offset = 0;
    let mut cut_duration = 0;
    // let mut length: u32;
    // let mut chunk_no: u32;
    let mut vec_result = Vec::<[u32; 2]>::new();
    while &offset != &copy.len() {
        let chunk_no = find_beginning(&mut original, &mut copy, offset, cut_duration)?;
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
    Ok(vec_result)
}
