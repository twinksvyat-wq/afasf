import UIKit
import AVFoundation

/// Haptic feedback tuned to fight events.
final class Haptics {
    private let light = UIImpactFeedbackGenerator(style: .light)
    private let medium = UIImpactFeedbackGenerator(style: .medium)
    private let heavy = UIImpactFeedbackGenerator(style: .heavy)
    private let rigid = UIImpactFeedbackGenerator(style: .rigid)
    private let note = UINotificationFeedbackGenerator()

    init() {
        [light, medium, heavy, rigid].forEach { $0.prepare() }
    }
    func tap() { light.impactOccurred(intensity: 0.6) }
    func hit(strength: Float, onPlayer: Bool) {
        let k = CGFloat(min(1, 0.35 + strength / 22))
        if onPlayer { heavy.impactOccurred(intensity: k) } else { medium.impactOccurred(intensity: k) }
    }
    func block() { rigid.impactOccurred(intensity: 0.5) }
    func knockdown() { heavy.impactOccurred(intensity: 1) }
    func ko() { note.notificationOccurred(.warning) }
    func success() { note.notificationOccurred(.success) }
    func error() { note.notificationOccurred(.error) }
}

/// Tiny procedural sound engine: every sound is synthesized at launch (no assets).
final class SoundEngine {
    private let engine = AVAudioEngine()
    private var players: [AVAudioPlayerNode] = []
    private var next = 0
    private let format = AVAudioFormat(standardFormatWithSampleRate: 44100, channels: 1)!
    private var buffers: [String: AVAudioPCMBuffer] = [:]
    private var ok = false

    init() {
        try? AVAudioSession.sharedInstance().setCategory(.ambient, options: [.mixWithOthers])
        try? AVAudioSession.sharedInstance().setActive(true)
        for _ in 0..<8 {
            let p = AVAudioPlayerNode()
            engine.attach(p)
            engine.connect(p, to: engine.mainMixerNode, format: format)
            players.append(p)
        }
        buffers["punch"] = synth(dur: 0.16) { t, n in
            let body = sin(2 * .pi * (95 - 220 * t) * t) * exp(-t * 32)
            return body * 0.9 + n * exp(-t * 90) * 0.55
        }
        buffers["kick"] = synth(dur: 0.22) { t, n in
            sin(2 * .pi * (70 - 120 * t) * t) * exp(-t * 20) + n * exp(-t * 70) * 0.5
        }
        buffers["block"] = synth(dur: 0.08) { t, n in n * exp(-t * 80) * 0.6 + sin(2 * .pi * 420 * t) * exp(-t * 60) * 0.2 }
        buffers["parry"] = synth(dur: 0.3) { t, _ in (sin(2 * .pi * 1400 * t) + sin(2 * .pi * 2100 * t) * 0.5) * exp(-t * 14) * 0.35 }
        buffers["thud"] = synth(dur: 0.35) { t, n in sin(2 * .pi * 48 * t) * exp(-t * 11) + n * exp(-t * 30) * 0.35 }
        buffers["bell"] = synth(dur: 1.8) { t, _ in
            let f = 880.0
            return (sin(2 * .pi * f * t) + 0.6 * sin(2 * .pi * f * 2.76 * t) + 0.4 * sin(2 * .pi * f * 5.4 * t)) * exp(-t * 2.2) * 0.35
        }
        buffers["ding"] = synth(dur: 0.6) { t, _ in (sin(2 * .pi * 1320 * t) + 0.5 * sin(2 * .pi * 3640 * t)) * exp(-t * 6) * 0.3 }
        buffers["coin"] = synth(dur: 0.25) { t, _ in sin(2 * .pi * (t < 0.07 ? 988 : 1319) * t) * exp(-t * 9) * 0.3 }
        do {
            try engine.start()
            players.forEach { $0.play() }
            ok = true
        } catch {
            ok = false
        }
    }

    private func synth(dur: Double, _ f: (Double, Double) -> Double) -> AVAudioPCMBuffer {
        let n = AVAudioFrameCount(dur * 44100)
        let b = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: n)!
        b.frameLength = n
        let ch = b.floatChannelData![0]
        var seed: UInt32 = 12345
        for i in 0..<Int(n) {
            seed = seed &* 1664525 &+ 1013904223
            let noise = Double(seed >> 8) / Double(1 << 24) * 2 - 1
            ch[i] = Float(max(-1, min(1, f(Double(i) / 44100, noise))))
        }
        return b
    }

    private func play(_ name: String, volume: Float = 1, rate: Float = 1) {
        guard ok, let b = buffers[name] else { return }
        let p = players[next]
        next = (next + 1) % players.count
        p.volume = volume
        p.scheduleBuffer(b, at: nil, options: [.interrupts], completionHandler: nil)
        if !p.isPlaying { p.play() }
        _ = rate
    }

    func hit(strength: Float, kick: Bool) { play(kick ? "kick" : "punch", volume: min(1, 0.4 + strength / 20)) }
    func block() { play("block", volume: 0.7) }
    func parry() { play("parry") }
    func thud(strength: Float) { play("thud", volume: min(1, strength / 6)) }
    func bell(short: Bool) { play(short ? "ding" : "bell") }
    func coin() { play("coin") }
}
