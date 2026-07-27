use std::env;
use std::process::ExitCode;
use std::time::Duration;

use mittens_engine::engine::ecs::component::{MirrorComponent, SecondaryMotionComponent};
use mittens_engine::engine::vr_perf::{VrPerfCase, VrPerfConfig};
use mittens_engine::{engine, engine::ecs::SignalEmitter, scripting, utils};

fn main() -> ExitCode {
    let perf = match parse_args() {
        Ok(perf) => perf,
        Err(error) => {
            eprintln!("{error}\n\n{}", usage());
            return ExitCode::from(2);
        }
    };

    mittens_engine::example_support::ensure_model_assets();
    utils::logger::init();
    // A path-aware evaluation is required so the scene can import its MMS preset module.
    let output = scripting::MeowMeowRunner::eval_file("examples/vtuber-secondary-motion.mms");
    for error in &output.errors {
        eprintln!("[mms] {error}");
    }
    assert!(
        output.errors.is_empty(),
        "MMS evaluation produced errors: {:?}",
        output.errors
    );

    let mut universe = engine::Universe::new(engine::ecs::World::default());
    let scope = engine::ecs::ComponentId::default();
    for intent in output.intents {
        universe.command_queue.push_intent_now(scope, intent);
    }
    universe.systems.process_commands(
        &mut universe.world,
        &mut universe.visuals,
        &mut universe.render_assets,
        &mut universe.command_queue,
    );

    if let Some(config) = perf {
        println!(
            "[vr-perf] preset={} avatar_xr=on mirror={} secondary_motion={} visualization={} warmup_seconds={:.3} sample_seconds={:.3}",
            config.case.as_str(),
            on_off(config.case.mirror()),
            on_off(config.case.secondary_motion()),
            on_off(config.case.visualization()),
            config.warmup.as_secs_f64(),
            config.sample.as_secs_f64(),
        );
        apply_perf_case(&mut universe, config.case);
        universe.configure_vr_perf(config);
    } else {
        universe.enable_repl();
    }

    match engine::Windowing::run_app(universe) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Windowing failed: {error:?}");
            ExitCode::FAILURE
        }
    }
}

fn apply_perf_case(universe: &mut engine::Universe, case: VrPerfCase) {
    let mirror_ids: Vec<_> = universe
        .world
        .all_components()
        .filter(|id| {
            universe
                .world
                .get_component_by_id_as::<MirrorComponent>(*id)
                .is_some()
        })
        .collect();
    let secondary_ids: Vec<_> = universe
        .world
        .all_components()
        .filter(|id| {
            universe
                .world
                .get_component_by_id_as::<SecondaryMotionComponent>(*id)
                .is_some()
        })
        .collect();

    if !case.mirror() {
        for component in mirror_ids {
            universe.command_queue.push_intent_now(
                component,
                engine::ecs::IntentValue::RemoveSubtree {
                    component_ids: vec![component],
                },
            );
        }
    }
    if !case.secondary_motion() {
        for component in secondary_ids {
            universe.command_queue.push_intent_now(
                component,
                engine::ecs::IntentValue::RemoveSubtree {
                    component_ids: vec![component],
                },
            );
        }
    } else if case.visualization() {
        for component in secondary_ids {
            universe
                .systems
                .spring_bone_visualization
                .set_request(component, vec![component]);
        }
    }

    universe.systems.process_commands(
        &mut universe.world,
        &mut universe.visuals,
        &mut universe.render_assets,
        &mut universe.command_queue,
    );
}

fn parse_args() -> Result<Option<VrPerfConfig>, String> {
    parse_args_from(env::args().skip(1))
}

fn parse_args_from(args: impl IntoIterator<Item = String>) -> Result<Option<VrPerfConfig>, String> {
    let mut args = args.into_iter();
    let mut case = None;
    let mut warmup_seconds = 5.0;
    let mut sample_seconds = 10.0;
    let mut saw_perf_option = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--vr-perf-case" => {
                saw_perf_option = true;
                let value = args
                    .next()
                    .ok_or_else(|| "--vr-perf-case requires a preset name".to_string())?;
                case = Some(
                    VrPerfCase::parse(&value)
                        .ok_or_else(|| format!("unknown VR performance preset: {value}"))?,
                );
            }
            "--vr-perf-warmup-seconds" => {
                saw_perf_option = true;
                warmup_seconds = parse_seconds("--vr-perf-warmup-seconds", args.next(), true)?;
            }
            "--vr-perf-sample-seconds" => {
                saw_perf_option = true;
                sample_seconds = parse_seconds("--vr-perf-sample-seconds", args.next(), false)?;
            }
            "--help" | "-h" => return Err(usage().to_string()),
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }

    if !saw_perf_option {
        return Ok(None);
    }
    let case = case.ok_or_else(|| {
        "VR performance timing options require --vr-perf-case <preset>".to_string()
    })?;
    Ok(Some(VrPerfConfig::new(
        case,
        Duration::from_secs_f64(warmup_seconds),
        Duration::from_secs_f64(sample_seconds),
    )))
}

fn parse_seconds(name: &str, value: Option<String>, allow_zero: bool) -> Result<f64, String> {
    let raw = value.ok_or_else(|| format!("{name} requires a number"))?;
    let seconds = raw
        .parse::<f64>()
        .map_err(|_| format!("{name} must be a number, got {raw:?}"))?;
    if !seconds.is_finite() || seconds < 0.0 || (!allow_zero && seconds == 0.0) {
        let expected = if allow_zero {
            "a finite non-negative number"
        } else {
            "a finite positive number"
        };
        return Err(format!("{name} must be {expected}, got {raw:?}"));
    }
    Ok(seconds)
}

fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

fn usage() -> &'static str {
    "Usage:
  cargo run --release --example vtuber-secondary-motion
  cargo run --release --example vtuber-secondary-motion -- \\
    --vr-perf-case <preset> \\
    [--vr-perf-warmup-seconds <seconds>] \\
    [--vr-perf-sample-seconds <seconds>]

Presets:
  avatar_no_spring_no_mirror
  avatar_no_spring_mirror
  avatar_spring_no_viz_mirror
  avatar_spring_viz_mirror"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn no_arguments_retains_interactive_mode() {
        assert!(parse_args_from(strings(&[])).unwrap().is_none());
    }

    #[test]
    fn parses_case_and_timing_overrides() {
        let config = parse_args_from(strings(&[
            "--vr-perf-case",
            "avatar_spring_viz_mirror",
            "--vr-perf-warmup-seconds",
            "0",
            "--vr-perf-sample-seconds",
            "2.5",
        ]))
        .unwrap()
        .unwrap();
        assert_eq!(config.case, VrPerfCase::AvatarSpringVizMirror);
        assert_eq!(config.warmup, Duration::ZERO);
        assert_eq!(config.sample, Duration::from_secs_f64(2.5));
    }

    #[test]
    fn timing_without_case_is_rejected() {
        let error = parse_args_from(strings(&["--vr-perf-sample-seconds", "1"])).unwrap_err();
        assert!(error.contains("--vr-perf-case"));
    }
}
