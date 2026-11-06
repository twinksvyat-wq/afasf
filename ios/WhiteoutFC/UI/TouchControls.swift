import UIKit
import SwiftUI

/// Multi-touch fight controls in UIKit for reliable simultaneous input:
/// floating joystick (left), action buttons (right), free camera drag (anywhere else).
final class TouchControlsView: UIView {
    var core: GameCore?

    private struct Pad {
        let id: Int32
        let label: String
        let sub: String
        let color: UIColor
        var radius: CGFloat
        var center: CGPoint = .zero
        let view: PadView
    }

    private var pads: [Pad] = []
    private var stickTouch: UITouch?
    private var stickOrigin = CGPoint.zero
    private var padTouches: [UITouch: Int32] = [:]
    private var camTouches: [UITouch: CGPoint] = [:]
    private let base = UIView()
    private let knob = UIView()
    private let stickRadius: CGFloat = 62
    private var restCenter = CGPoint.zero

    override init(frame: CGRect) {
        super.init(frame: frame)
        isMultipleTouchEnabled = true
        backgroundColor = .clear
        base.backgroundColor = UIColor.black.withAlphaComponent(0.08)
        base.layer.borderColor = UIColor.black.withAlphaComponent(0.25).cgColor
        base.layer.borderWidth = 2
        knob.backgroundColor = UIColor.black.withAlphaComponent(0.75)
        knob.layer.shadowColor = UIColor.black.cgColor
        knob.layer.shadowOpacity = 0.25
        knob.layer.shadowRadius = 6
        [base, knob].forEach { $0.isUserInteractionEnabled = false; addSubview($0) }
        let defs: [(Int32, String, String, UIColor, CGFloat)] = [
            (Btn.punch, "УДАР", "руки", UIColor(red: 0.95, green: 0.18, blue: 0.2, alpha: 1), 46),
            (Btn.kick, "НОГА", "пинок", UIColor(red: 1.0, green: 0.55, blue: 0.1, alpha: 1), 40),
            (Btn.grab, "ЗАХВАТ", "бросок", UIColor(red: 0.2, green: 0.45, blue: 1.0, alpha: 1), 34),
            (Btn.block, "БЛОК", "держи", UIColor(white: 0.18, alpha: 1), 38),
            (Btn.special, "СУПЕР", "", UIColor(red: 1.0, green: 0.78, blue: 0.1, alpha: 1), 32),
        ]
        for d in defs {
            let v = PadView(label: d.1, sub: d.2, color: d.3)
            v.isUserInteractionEnabled = false
            addSubview(v)
            pads.append(Pad(id: d.0, label: d.1, sub: d.2, color: d.3, radius: d.4, view: v))
        }
    }

    required init?(coder: NSCoder) { fatalError() }

    override func layoutSubviews() {
        super.layoutSubviews()
        let s = safeAreaInsets
        let w = bounds.width, h = bounds.height
        let scale: CGFloat = min(1.25, max(0.85, h / 390))
        let anchor = CGPoint(x: w - s.right - 120 * scale, y: h - s.bottom - 92 * scale)
        let offsets: [Int32: CGPoint] = [
            Btn.punch: CGPoint(x: 18, y: 0),
            Btn.kick: CGPoint(x: -78, y: 30),
            Btn.grab: CGPoint(x: -52, y: -72),
            Btn.block: CGPoint(x: 72, y: -78),
            Btn.special: CGPoint(x: -10, y: -150),
        ]
        for i in pads.indices {
            let o = offsets[pads[i].id] ?? .zero
            pads[i].center = CGPoint(x: anchor.x + o.x * scale, y: anchor.y + o.y * scale)
            let r = pads[i].radius * scale
            pads[i].view.frame = CGRect(x: pads[i].center.x - r, y: pads[i].center.y - r, width: r * 2, height: r * 2)
            pads[i].view.layer.cornerRadius = r
        }
        restCenter = CGPoint(x: s.left + 120 * scale, y: h - s.bottom - 100 * scale)
        if stickTouch == nil { placeStick(at: restCenter, knobAt: restCenter, active: false) }
    }

    private func placeStick(at c: CGPoint, knobAt k: CGPoint, active: Bool) {
        let r = stickRadius
        base.frame = CGRect(x: c.x - r, y: c.y - r, width: r * 2, height: r * 2)
        base.layer.cornerRadius = r
        knob.frame = CGRect(x: k.x - 28, y: k.y - 28, width: 56, height: 56)
        knob.layer.cornerRadius = 28
        base.alpha = active ? 1 : 0.55
        knob.alpha = active ? 1 : 0.5
    }

    /// Called from the HUD with the special-meter state.
    func setSpecial(fill: Float, ready: UInt32) {
        guard let i = pads.firstIndex(where: { $0.id == Btn.special }) else { return }
        let v = pads[i].view
        v.setFill(CGFloat(fill))
        v.setTitle(ready == 2 ? "ФИНИШ" : "СУПЕР", glowing: ready > 0)
    }

    private func pad(at p: CGPoint) -> Int32? {
        for pd in pads {
            let d = hypot(p.x - pd.center.x, p.y - pd.center.y)
            if d < pd.view.bounds.width * 0.5 + 14 { return pd.id }
        }
        return nil
    }

    private func setPad(_ id: Int32, down: Bool) {
        core?.button(id, down)
        if let i = pads.firstIndex(where: { $0.id == id }) {
            pads[i].view.setPressed(down)
        }
        if down { core?.haptics.tap() }
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        for t in touches {
            let p = t.location(in: self)
            if let id = pad(at: p) {
                padTouches[t] = id
                setPad(id, down: true)
            } else if p.x < bounds.width * 0.45 && p.y > bounds.height * 0.3 && stickTouch == nil {
                stickTouch = t
                stickOrigin = p
                placeStick(at: p, knobAt: p, active: true)
            } else {
                camTouches[t] = p
            }
        }
    }

    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        for t in touches {
            let p = t.location(in: self)
            if t == stickTouch {
                var d = CGPoint(x: p.x - stickOrigin.x, y: p.y - stickOrigin.y)
                let len = hypot(d.x, d.y)
                if len > stickRadius {
                    // drag the base along so the stick never "runs out"
                    let k = (len - stickRadius) / len
                    stickOrigin.x += d.x * k
                    stickOrigin.y += d.y * k
                    d = CGPoint(x: p.x - stickOrigin.x, y: p.y - stickOrigin.y)
                }
                placeStick(at: stickOrigin, knobAt: p, active: true)
                core?.stick(Float(d.x / stickRadius), Float(-d.y / stickRadius))
            } else if let last = camTouches[t] {
                core?.orbit(Float(-(p.x - last.x) * 0.006), Float((p.y - last.y) * 0.003))
                camTouches[t] = p
            } else if let id = padTouches[t] {
                // sliding a finger between buttons (e.g. punch -> kick)
                if let nid = pad(at: p), nid != id {
                    setPad(id, down: false)
                    padTouches[t] = nid
                    setPad(nid, down: true)
                }
            }
        }
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) { finish(touches) }
    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) { finish(touches) }

    private func finish(_ touches: Set<UITouch>) {
        for t in touches {
            if t == stickTouch {
                stickTouch = nil
                core?.stick(0, 0)
                placeStick(at: restCenter, knobAt: restCenter, active: false)
            } else if let id = padTouches.removeValue(forKey: t) {
                setPad(id, down: false)
            } else {
                camTouches.removeValue(forKey: t)
            }
        }
    }

    func releaseAll() {
        for (_, id) in padTouches { setPad(id, down: false) }
        padTouches.removeAll()
        camTouches.removeAll()
        stickTouch = nil
        core?.stick(0, 0)
        placeStick(at: restCenter, knobAt: restCenter, active: false)
    }
}

/// Round action button with label, press state and an optional fill meter.
final class PadView: UIView {
    private let title = UILabel()
    private let subtitle = UILabel()
    private let fill = CALayer()
    private let color: UIColor
    private var fillAmount: CGFloat = 1

    init(label: String, sub: String, color: UIColor) {
        self.color = color
        super.init(frame: .zero)
        backgroundColor = color.withAlphaComponent(0.88)
        layer.borderColor = UIColor.white.withAlphaComponent(0.9).cgColor
        layer.borderWidth = 3
        layer.shadowColor = UIColor.black.cgColor
        layer.shadowOpacity = 0.25
        layer.shadowRadius = 8
        layer.shadowOffset = CGSize(width: 0, height: 4)
        clipsToBounds = false
        fill.backgroundColor = UIColor.white.withAlphaComponent(0.35).cgColor
        layer.insertSublayer(fill, at: 0)
        title.text = label
        title.textColor = .white
        title.textAlignment = .center
        title.adjustsFontSizeToFitWidth = true
        title.minimumScaleFactor = 0.5
        subtitle.text = sub.uppercased()
        subtitle.textColor = UIColor.white.withAlphaComponent(0.75)
        subtitle.textAlignment = .center
        addSubview(title)
        addSubview(subtitle)
    }
    required init?(coder: NSCoder) { fatalError() }

    override func layoutSubviews() {
        super.layoutSubviews()
        let r = bounds.width / 2
        let f = UIFont.systemFont(ofSize: r * 0.42, weight: .black)
        title.font = UIFont(descriptor: f.fontDescriptor.withSymbolicTraits(.traitItalic) ?? f.fontDescriptor, size: r * 0.42)
        subtitle.font = .systemFont(ofSize: max(8, r * 0.22), weight: .bold)
        let hasSub = !(subtitle.text ?? "").isEmpty
        title.frame = CGRect(x: 4, y: bounds.midY - r * (hasSub ? 0.34 : 0.24), width: bounds.width - 8, height: r * 0.5)
        subtitle.frame = CGRect(x: 4, y: title.frame.maxY - 2, width: bounds.width - 8, height: r * 0.3)
        updateFill()
    }

    private func updateFill() {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        let h = bounds.height * (1 - fillAmount)
        fill.frame = CGRect(x: 0, y: 0, width: bounds.width, height: h)
        fill.cornerRadius = 0
        layer.masksToBounds = false
        CATransaction.commit()
    }

    func setFill(_ v: CGFloat) {
        fillAmount = max(0, min(1, v))
        alpha = 0.55 + 0.45 * fillAmount
        updateFill()
    }

    func setTitle(_ t: String, glowing: Bool) {
        if title.text != t { title.text = t }
        layer.shadowColor = glowing ? color.cgColor : UIColor.black.cgColor
        layer.shadowOpacity = glowing ? 0.95 : 0.25
        layer.shadowRadius = glowing ? 18 : 8
        if glowing && layer.animation(forKey: "pulse") == nil {
            let a = CABasicAnimation(keyPath: "transform.scale")
            a.fromValue = 1.0
            a.toValue = 1.1
            a.duration = 0.45
            a.autoreverses = true
            a.repeatCount = .infinity
            layer.add(a, forKey: "pulse")
        } else if !glowing {
            layer.removeAnimation(forKey: "pulse")
        }
    }

    func setPressed(_ p: Bool) {
        UIView.animate(withDuration: 0.06) {
            self.transform = p ? CGAffineTransform(scaleX: 0.88, y: 0.88) : .identity
            self.backgroundColor = p ? self.color : self.color.withAlphaComponent(0.88)
        }
    }
}

struct TouchControls: UIViewRepresentable {
    @ObservedObject var core: GameCore
    func makeUIView(context: Context) -> TouchControlsView {
        let v = TouchControlsView(frame: .zero)
        v.core = core
        return v
    }
    func updateUIView(_ v: TouchControlsView, context: Context) {
        v.setSpecial(fill: core.hud.player.special, ready: core.hud.finisherReady)
        if core.paused || core.hud.phase >= 3 { v.releaseAll() }
    }
}
