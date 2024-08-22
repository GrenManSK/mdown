use rodio::{ Decoder, OutputStream, Sink, Source };
use std::{ io::Cursor, thread, time::Duration };

use crate::{ args::ARGS_MUSIC, resolute::{ MUSIC_END, MUSIC_STAGE } };

include!(concat!(env!("OUT_DIR"), "/no_mp3.rs"));

#[cfg(music_m1)]
include!(concat!(env!("OUT_DIR"), "/m1_combat_mp3.rs"));
#[cfg(music_m1)]
include!(concat!(env!("OUT_DIR"), "/m1_end_mp3.rs"));
#[cfg(music_m1)]
include!(concat!(env!("OUT_DIR"), "/m1_start_c_mp3.rs"));
#[cfg(music_m1)]
include!(concat!(env!("OUT_DIR"), "/m1_stealth_mp3.rs"));

#[cfg(music_m2)]
include!(concat!(env!("OUT_DIR"), "/m2_combat_mp3.rs"));
#[cfg(music_m2)]
include!(concat!(env!("OUT_DIR"), "/m2_end_mp3.rs"));
#[cfg(music_m2)]
include!(concat!(env!("OUT_DIR"), "/m2_start_c_mp3.rs"));
#[cfg(music_m2)]
include!(concat!(env!("OUT_DIR"), "/m2_stealth_mp3.rs"));

#[cfg(music_m3)]
include!(concat!(env!("OUT_DIR"), "/m3_combat_mp3.rs"));
#[cfg(music_m3)]
include!(concat!(env!("OUT_DIR"), "/m3_end_mp3.rs"));
#[cfg(music_m3)]
include!(concat!(env!("OUT_DIR"), "/m3_start_c_mp3.rs"));

#[cfg(music_m4)]
include!(concat!(env!("OUT_DIR"), "/m4_combat_mp3.rs"));
#[cfg(music_m4)]
include!(concat!(env!("OUT_DIR"), "/m4_end_mp3.rs"));
#[cfg(music_m4)]
include!(concat!(env!("OUT_DIR"), "/m4_start_c_mp3.rs"));
#[cfg(music_m4)]
include!(concat!(env!("OUT_DIR"), "/m4_stealth_mp3.rs"));

enum State {
    Initial,
    StealthPlaying,
    CombatPlaying,
}

pub(crate) fn start() {
    let (_stream, stream_handle) = match OutputStream::try_default() {
        Ok(stream) => stream,
        Err(_) => {
            eprintln!("Couldn't open OutputStream (music)");
            return;
        }
    };

    let mut state = State::Initial;
    let mut stealth_sink = None;
    let mut combat_sink = None;

    let music_pack;
    match ARGS_MUSIC.clone() {
        Some(s) => {
            music_pack = match s.clone() {
                Some(value) => {
                    match value.parse::<u32>() {
                        Ok(value) => value.clone(),
                        Err(_) => 1,
                    }
                }
                None => 1,
            };
        }
        None => {
            return;
        }
    }

    loop {
        let temp = MUSIC_STAGE.lock().clone();
        match temp.as_str() {
            "init" => {
                if let State::Initial = state {
                    let music = match music_pack {
                        #[cfg(music_m1)]
                        1 => M1_STEALTH_MP3,
                        #[cfg(music_m2)]
                        2 => M2_STEALTH_MP3,
                        #[cfg(music_m3)]
                        3 => {
                            state = State::StealthPlaying;
                            continue;
                        }
                        #[cfg(music_m4)]
                        4 => M4_STEALTH_MP3,
                        _ => NO_MP3,
                    };
                    let cursor = Cursor::new(music);
                    let sink = match Sink::try_new(&stream_handle) {
                        Ok(sink) => sink,
                        Err(err) => {
                            eprintln!("Error creating Sink: {}", err);
                            return;
                        }
                    };
                    let source = match Decoder::new(cursor) {
                        Ok(source) => source,
                        Err(err) => {
                            eprintln!("Error creating decoder: {}", err);
                            return;
                        }
                    };
                    sink.append(source.repeat_infinite());
                    stealth_sink = Some(sink);

                    state = State::StealthPlaying;
                }
            }
            "start" => {
                if let State::StealthPlaying = state {
                    if let Some(sink) = stealth_sink.take() {
                        sink.stop();
                    }

                    let music = match music_pack {
                        #[cfg(music_m1)]
                        1 => M1_START_C_MP3,
                        #[cfg(music_m2)]
                        2 => M2_START_C_MP3,
                        #[cfg(music_m3)]
                        3 => M3_START_C_MP3,
                        #[cfg(music_m4)]
                        4 => M4_START_C_MP3,
                        _ => NO_MP3,
                    };
                    let cursor = Cursor::new(music);
                    let start_sink = match Sink::try_new(&stream_handle) {
                        Ok(sink) => sink,
                        Err(err) => {
                            eprintln!("Error creating Sink: {}", err);
                            return;
                        }
                    };
                    let source = match Decoder::new(cursor) {
                        Ok(source) => source,
                        Err(err) => {
                            eprintln!("Error creating decoder: {}", err);
                            return;
                        }
                    };
                    start_sink.append(source);

                    start_sink.sleep_until_end();
                    start_sink.stop();

                    let music = match music_pack {
                        #[cfg(music_m1)]
                        1 => M1_COMBAT_MP3,
                        #[cfg(music_m2)]
                        2 => M2_COMBAT_MP3,
                        #[cfg(music_m3)]
                        3 => M3_COMBAT_MP3,
                        #[cfg(music_m4)]
                        4 => M4_COMBAT_MP3,
                        _ => NO_MP3,
                    };
                    let cursor = Cursor::new(music);
                    let sink = match Sink::try_new(&stream_handle) {
                        Ok(sink) => sink,
                        Err(err) => {
                            eprintln!("Error creating Sink: {}", err);
                            return;
                        }
                    };
                    let source = match Decoder::new(cursor) {
                        Ok(source) => source,
                        Err(err) => {
                            eprintln!("Error creating decoder: {}", err);
                            return;
                        }
                    };
                    sink.append(source.repeat_infinite());

                    combat_sink = Some(sink);

                    state = State::CombatPlaying;
                }
            }
            "end" => {
                if let Some(sink) = &combat_sink {
                    let fade_duration = Duration::from_secs(2);
                    let fade_steps = 20;
                    let sleep_duration = fade_duration / fade_steps;
                    let mut current_volume = 1.0;
                    let mut end_sink = match Sink::try_new(&stream_handle) {
                        Ok(sink) => sink,
                        Err(err) => {
                            eprintln!("Error creating Sink: {}", err);
                            return;
                        }
                    };
                    for i in 0..fade_steps {
                        current_volume -= 0.75 / (fade_steps as f32);
                        sink.set_volume(current_volume);

                        if i == fade_steps - 10 {
                            let music = match music_pack {
                                #[cfg(music_m1)]
                                1 => M1_END_MP3,
                                #[cfg(music_m2)]
                                2 => M2_END_MP3,
                                #[cfg(music_m3)]
                                3 => M3_END_MP3,
                                #[cfg(music_m4)]
                                4 => M4_END_MP3,
                                _ => NO_MP3,
                            };
                            let end_cursor = Cursor::new(music);
                            end_sink = match Sink::try_new(&stream_handle) {
                                Ok(sink) => sink,
                                Err(err) => {
                                    eprintln!("Error creating Sink: {}", err);
                                    return;
                                }
                            };
                            let end_source = match Decoder::new(end_cursor) {
                                Ok(cursor) => cursor,
                                Err(err) => {
                                    eprintln!("Error creating cursor: {}", err);
                                    return;
                                }
                            };
                            end_sink.append(end_source);
                        }

                        thread::sleep(sleep_duration);
                    }
                    sink.stop();
                    end_sink.sleep_until_end();
                    end_sink.stop();
                } else {
                    if let Some(sink) = stealth_sink.take() {
                        sink.stop();
                    }

                    if !combat_sink.is_some() {
                        let music = match music_pack {
                            #[cfg(music_m1)]
                            1 => M1_END_MP3,
                            #[cfg(music_m2)]
                            2 => M2_END_MP3,
                            #[cfg(music_m3)]
                            3 => M3_END_MP3,
                            #[cfg(music_m4)]
                            4 => M4_END_MP3,
                            _ => NO_MP3,
                        };
                        let end_cursor = Cursor::new(music);
                        let end_sink = match Sink::try_new(&stream_handle) {
                            Ok(sink) => sink,
                            Err(err) => {
                                eprintln!("Error creating Sink: {}", err);
                                return;
                            }
                        };
                        let end_source = match Decoder::new(end_cursor) {
                            Ok(cursor) => cursor,
                            Err(err) => {
                                eprintln!("Error creating cursor: {}", err);
                                return;
                            }
                        };
                        end_sink.append(end_source);

                        end_sink.sleep_until_end();
                        end_sink.stop();
                    }

                    *MUSIC_STAGE.lock() = String::new();
                }
                if *MUSIC_END.lock() {
                    std::process::exit(0);
                }
                return;
            }
            _ => {}
        }
    }
}
