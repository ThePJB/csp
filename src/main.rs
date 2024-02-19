use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use std::f32::consts::PI;

// csp 220,+3,+2,+2,+1,+2,+2

pub struct SampleRequestOptions {
    pub sample_rate: f32,
    pub nchannels: usize,
    pub sample: u64,
    pub notes: Vec<f32>,
}

impl SampleRequestOptions {
    fn new(nchannels: usize, sample_rate: f32, notes: Vec<f32>) -> Self {
        SampleRequestOptions {
            sample_rate,
            nchannels,
            sample: 0,
            notes,
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        println!("")
    }

    if args.len() != 2 {
        panic!("no melody argument, e.g. 220,+3,+2,+2,+1,+2,+2");
    }
    let mut curr_freq = 220.0;
    let mut notes = Vec::new();
    for sym_text in args[1].split(",") {
        if let Some(s) = sym_text.strip_prefix('+') {
            if let Ok(s) = s.parse::<f32>() {
                curr_freq *= 2.0f32.powf(s/12.0);
                notes.push(curr_freq);
            }
            continue;
        }
        if let Some(s) = sym_text.strip_prefix('-') {
            if let Ok(s) = s.parse::<f32>() {
                curr_freq /= 2.0f32.powf(s/12.0);
                notes.push(curr_freq);
            }
            continue;
        }
        if let Ok(f) = sym_text.parse() {
            notes.push(f);
            continue;
        }
    }

    let stream = stream_setup_for(sample_next, notes).expect("no can make stream");
    stream.play().unwrap();
    loop {}
}

pub fn sample_next(o: &mut SampleRequestOptions) -> f32 {
    let time_per_note = 10000;
    let window_time = 1000;

    o.sample += 1;

    let note_samp = o.sample % time_per_note;

    let window_phase = if note_samp < window_time {
        note_samp as f32 / window_time as f32 * PI / 2.0
    } else if note_samp > time_per_note - window_time {
        PI / 2.0 + (note_samp - (time_per_note - window_time)) as f32 / window_time as f32 * PI / 2.0
    } else {
        PI / 2.0
    };
    // idk why still popping I thought my idea was sound
    // maybe float shit is fucking it?? do everything in samples

    // now when playing determine which note we are currently playing
    // fuck yea this is it essentially just need window for pops / adsr or whatever
    // and more operators
    // how repeat n operators: .4
    // so you could be like .4,.,.
    // ah but hard if u wanna change it
    // but yea ideally some kind of expressive formal system
    // maybe if repeat quantity was like always that many notes, if you could do 3 of last 4 then specify a new one

    if let Some(curr_note) = o.notes.get((o.sample / time_per_note) as usize) {
        return (o.sample as f32 / o.sample_rate * *curr_note * 2.0 * PI).sin() * 0.1 * window_phase.sin()
    } else {
        println!("tune end");
        std::process::exit(0)
    }    
}

pub fn stream_setup_for<F>(on_sample: F, notes: Vec<f32>) -> Result<cpal::Stream, anyhow::Error>
where
    F: FnMut(&mut SampleRequestOptions) -> f32 + std::marker::Send + 'static + Copy,
{
    let (_host, device, config) = host_device_setup()?;

    match config.sample_format() {
        cpal::SampleFormat::F32 => stream_make::<f32, _>(&device, &config.into(), on_sample, notes),
        cpal::SampleFormat::I16 => stream_make::<i16, _>(&device, &config.into(), on_sample, notes),
        cpal::SampleFormat::U16 => stream_make::<u16, _>(&device, &config.into(), on_sample, notes),
    }
}

pub fn host_device_setup(
) -> Result<(cpal::Host, cpal::Device, cpal::SupportedStreamConfig), anyhow::Error> {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::Error::msg("Default output device is not available"))?;
    println!("Output device : {}", device.name()?);

    let config = device.default_output_config()?;
    println!("Default output config : {:?}", config);

    Ok((host, device, config))
}

pub fn stream_make<T, F>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    on_sample: F,
    notes: Vec<f32>,
) -> Result<cpal::Stream, anyhow::Error>
where
    T: cpal::Sample,
    F: FnMut(&mut SampleRequestOptions) -> f32 + std::marker::Send + 'static + Copy,
{
    let mut request = SampleRequestOptions::new(config.channels as usize, config.sample_rate.0 as f32, notes);

    let err_fn = |err| eprintln!("Error building output sound stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
            on_window(output, &mut request, on_sample)
        },
        err_fn,
    )?;

    Ok(stream)
}

fn on_window<T, F>(output: &mut [T], request: &mut SampleRequestOptions, mut on_sample: F)
where
    T: cpal::Sample,
    F: FnMut(&mut SampleRequestOptions) -> f32 + std::marker::Send + 'static,
{
    for frame in output.chunks_mut(request.nchannels) {
        let value: T = cpal::Sample::from::<f32>(&on_sample(request));
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}