RendererSettings { window_size(900, 650) }
BGC.rgba(0.03, 0.04, 0.07, 1.0)
Clock.bpm(120)
T.position(0.0, 1.4, 5.0) { C3D { Pointer {} } }
AudioOutput.off()

let oscillator = AudioOscillator.saw() { frequency(110) amplitude(0.25) enabled(false) }
let band_pass = AudioBandPassFilter.new(500.0, 1.25, 0.2)
AudioOutput {
    AudioLimiter.new(5.0, 80.0, 0.85) {
        AudioGain.new(0.35) {
            band_pass { oscillator }
        }
    }
}

let meter = T { R.cube() { C.rgba(0.2, 0.9, 0.65, 1.0) } }
meter
Animation.looping().length(4.0) {
    Keyframe.at(0.0) {
        band_pass.set_center_hz(240.0)
        meter.set_color([0.2, 0.9, 0.65, 1.0])
        MusicNote.a(2, 0.75, oscillator)
    }
    Keyframe.at(1.0) {
        band_pass.set_center_hz(600.0)
        meter.set_color([0.3, 0.55, 1.0, 1.0])
        MusicNote.e(3, 0.75, oscillator)
    }
    Keyframe.at(2.0) {
        band_pass.set_center_hz(1200.0)
        meter.set_color([1.0, 0.35, 0.55, 1.0])
        MusicNote.a(3, 0.75, oscillator)
    }
    Keyframe.at(3.0) { band_pass.set_center_hz(400.0) }
}
