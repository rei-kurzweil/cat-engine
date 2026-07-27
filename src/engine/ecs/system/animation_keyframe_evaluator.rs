use crate::engine::ecs::component::{KeyframeComponent, MusicNoteComponent};
use crate::engine::ecs::{ComponentId, IntentValue, RxWorld, SignalEmitter, World};
use crate::scripting::world_evaluator::{RuntimeClosureExecMode, eval_runtime_closure};

#[derive(Debug, Default)]
pub(crate) struct AnimationKeyframeEvaluator;

impl AnimationKeyframeEvaluator {
    pub(crate) fn evaluate_audio_due_keyframe(
        &self,
        world: &mut World,
        rx: &mut RxWorld,
        kf_id: ComponentId,
        kf_global_beat: f64,
    ) {
        let runtime_closure = world
            .get_component_by_id_as::<KeyframeComponent>(kf_id)
            .and_then(|kf| kf.callback.clone());

        if let Some(runtime_closure) = runtime_closure {
            if let Err(error) = eval_runtime_closure(
                &runtime_closure,
                None,
                Some(world),
                Some(rx),
                Some(kf_id),
                RuntimeClosureExecMode::KeyframeAudioOnly {
                    beat_context: kf_global_beat,
                },
            ) {
                eprintln!(
                    "[AnimationSystem] keyframe runtime closure audio lookahead failed for {kf_id:?}: {error}"
                );
            }
        }

        fire_music_note_children(world, rx, kf_id, Some(kf_global_beat));
    }

    pub(crate) fn evaluate_visual_due_keyframe(
        &self,
        world: &mut World,
        rx: &mut RxWorld,
        kf_id: ComponentId,
        beat_now: f64,
        audio_already_scheduled_this_cycle: bool,
    ) {
        let runtime_closure = world
            .get_component_by_id_as::<KeyframeComponent>(kf_id)
            .and_then(|kf| kf.callback.clone());

        if let Some(runtime_closure) = runtime_closure {
            if let Err(error) = eval_runtime_closure(
                &runtime_closure,
                None,
                Some(world),
                Some(rx),
                Some(kf_id),
                RuntimeClosureExecMode::KeyframeVisualOnly,
            ) {
                eprintln!(
                    "[AnimationSystem] keyframe runtime closure failed for {kf_id:?}: {error}"
                );
            }
        }

        if !audio_already_scheduled_this_cycle {
            fire_music_note_children(world, rx, kf_id, Some(beat_now));
        }
    }
}

fn fire_music_note_children(
    world: &mut World,
    rx: &mut RxWorld,
    kf_id: ComponentId,
    beat_context: Option<f64>,
) {
    let note_ids: Vec<ComponentId> = world
        .children_of(kf_id)
        .iter()
        .copied()
        .filter(|&cid| {
            world
                .get_component_by_id_as::<MusicNoteComponent>(cid)
                .is_some()
        })
        .collect();

    for note_cid in note_ids {
        let note = match world.get_component_by_id_as::<MusicNoteComponent>(note_cid) {
            Some(mn) => mn.note,
            None => continue,
        };
        rx.push_intent_now(
            note_cid,
            IntentValue::AudioSchedulePlay {
                component_id: note_cid,
                beat_offset: 0.0,
                beat_context,
                note: Some(note),
                gain: None,
                rate: None,
                duration: None,
            },
        );
    }
}
