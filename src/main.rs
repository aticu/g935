use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use clap::StructOpt;
use g935::Headset;

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// run in continuous mode
    RunContinuous,
    /// return the battery level
    GetBatteryLevel,
}

#[derive(clap::Parser, Debug)]
struct Args {
    /// how verbose the program should be
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,
    /// whether the program should be silent
    #[clap(short, long)]
    silent: bool,
    /// the action to perform
    #[clap(subcommand)]
    command: Command,
}

fn main() {
    let args = Args::parse();

    {
        let level_filter = match (args.silent, args.verbose) {
            (true, _) => LevelFilter::Off,
            (false, 0) => LevelFilter::Warn,
            (false, 1) => LevelFilter::Info,
            (false, 2) => LevelFilter::Debug,
            (false, _) => LevelFilter::Trace,
        };

        use simplelog::*;
        TermLogger::init(
            level_filter,
            Config::default(),
            TerminalMode::Stderr,
            ColorChoice::Auto,
        )
        .unwrap();
    }

    let mut headset = Headset::open().unwrap();

    match args.command {
        Command::GetBatteryLevel => match headset.get_battery_status() {
            Ok(status) => {
                println!("{} {}", status.charging_status, status.charge);
            }
            Err(err) => {
                log::error!("could not read battery status: {err}");
                std::process::exit(1);
            }
        },
        Command::RunContinuous => {
            let mut config = g935::config::Config::default();
            let mut old_button_state = g935::ButtonState::default();
            let battery_lights_start = Rc::new(Cell::new(None));
            let battery_lights_start2 = Rc::clone(&battery_lights_start);

            config.set_button_handler(Some(Box::new(move |config, headset, state| {
                if state.mic_flipped_up(&old_button_state) {
                    std::process::Command::new("amixer")
                        .arg("set")
                        .arg("Capture")
                        .arg("nocap")
                        .output()
                        .ok();
                }
                if state.mic_flipped_down(&old_button_state) {
                    std::process::Command::new("amixer")
                        .arg("set")
                        .arg("Capture")
                        .arg("cap")
                        .output()
                        .ok();
                }

                if state.g1_pressed(&old_button_state) {
                    std::process::Command::new("playerctl")
                        .arg("play-pause")
                        .output()
                        .ok();
                }
                if state.g2_pressed(&old_button_state) {
                    std::process::Command::new("playerctl")
                        .arg("next")
                        .output()
                        .ok();
                }
                if state.g3_pressed(&old_button_state) {
                    std::process::Command::new("playerctl")
                        .arg("previous")
                        .output()
                        .ok();
                }

                if state.scroll_up() {
                    std::process::Command::new("pactl")
                        .arg("set-sink-volume")
                        .arg("@DEFAULT_SINK@")
                        .arg("+2%")
                        .output()
                        .ok();
                }
                if state.scroll_down() {
                    std::process::Command::new("pactl")
                        .arg("set-sink-volume")
                        .arg("@DEFAULT_SINK@")
                        .arg("-2%")
                        .output()
                        .ok();
                }

                if state.mute_button_pressed() {
                    match headset.get_battery_status() {
                        Ok(battery_status) => {
                            let percent = (battery_status.charge * 2.55).round() as u8;

                            battery_lights_start.set(Some(Instant::now()));
                            config.set_side_light_effect(g935::lights::Effect::Static {
                                red: 255 - percent,
                                green: percent,
                                blue: 0,
                            });
                        }
                        Err(err) => log::warn!("failed to get battery status: {err}"),
                    }
                }

                old_button_state = state;
            })));

            config.set_periodic_handler(Some(Box::new(move |config, _| {
                let battery_lights_start = battery_lights_start2.get();

                if let Some(start) = battery_lights_start {
                    if start.elapsed() >= Duration::from_millis(1000) {
                        battery_lights_start2.set(None);
                        config.set_side_light_effect(g935::lights::Effect::Off);
                    }
                }
            })));

            headset.run_with_config(config);
        }
    }
}
