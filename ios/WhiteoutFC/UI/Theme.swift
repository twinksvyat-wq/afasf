import SwiftUI

enum Theme {
    static let ink = Color(red: 0.06, green: 0.06, blue: 0.08)
    static let paper = Color(red: 0.972, green: 0.973, blue: 0.978)
    static let red = Color(red: 0.95, green: 0.16, blue: 0.18)
    static let gold = Color(red: 1.0, green: 0.76, blue: 0.1)
    static let blue = Color(red: 0.2, green: 0.45, blue: 1.0)
    static let green = Color(red: 0.2, green: 0.8, blue: 0.4)
    static let muted = Color(red: 0.45, green: 0.46, blue: 0.5)

    static func display(_ size: CGFloat) -> Font {
        .system(size: size, weight: .black, design: .default).italic()
    }
    static func label(_ size: CGFloat, _ weight: Font.Weight = .bold) -> Font {
        .system(size: size, weight: weight, design: .default)
    }
}

/// Parallelogram — the game's signature shape.
struct Slant: Shape {
    var skew: CGFloat = 14
    func path(in r: CGRect) -> Path {
        var p = Path()
        p.move(to: CGPoint(x: r.minX + skew, y: r.minY))
        p.addLine(to: CGPoint(x: r.maxX, y: r.minY))
        p.addLine(to: CGPoint(x: r.maxX - skew, y: r.maxY))
        p.addLine(to: CGPoint(x: r.minX, y: r.maxY))
        p.closeSubpath()
        return p
    }
}

struct SlantButton: View {
    let title: String
    var subtitle: String? = nil
    var color: Color = Theme.ink
    var textColor: Color = .white
    var height: CGFloat = 52
    var size: CGFloat = 24
    var enabled = true
    let action: () -> Void
    @State private var pressed = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 10) {
                Text(title)
                    .font(Theme.display(size))
                    .fontWidth(.condensed)
                    .foregroundColor(textColor)
                if let s = subtitle {
                    Text(s)
                        .font(Theme.label(12, .heavy))
                        .foregroundColor(textColor.opacity(0.7))
                }
                Spacer(minLength: 0)
            }
            .padding(.horizontal, 26)
            .frame(height: height)
            .background(Slant().fill(color).shadow(color: .black.opacity(0.18), radius: 8, x: 0, y: 5))
            .contentShape(Slant())
            .opacity(enabled ? 1 : 0.4)
        }
        .buttonStyle(PressScale())
        .disabled(!enabled)
    }
}

struct PressScale: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed ? 0.95 : 1)
            .animation(.spring(response: 0.2, dampingFraction: 0.6), value: configuration.isPressed)
    }
}

struct CoinBadge: View {
    let coins: UInt32
    var body: some View {
        HStack(spacing: 6) {
            Circle().fill(Theme.gold).frame(width: 18, height: 18)
                .overlay(Text("₵").font(.system(size: 11, weight: .black)).foregroundColor(Theme.ink))
            Text("\(coins)")
                .font(Theme.display(20))
                .monospacedDigit()
                .foregroundColor(Theme.ink)
                .contentTransition(.numericText())
        }
        .padding(.horizontal, 14).padding(.vertical, 6)
        .background(Capsule().fill(.white).shadow(color: .black.opacity(0.12), radius: 6, y: 3))
        .animation(.spring(), value: coins)
    }
}

/// Left-side menu panel used by every sub-screen; the 3D stage stays visible to the right.
struct SidePanel<Content: View>: View {
    let title: String
    let back: () -> Void
    @ViewBuilder let content: Content
    var body: some View {
        GeometryReader { geo in
            HStack(spacing: 0) {
                VStack(alignment: .leading, spacing: 12) {
                    HStack(spacing: 14) {
                        Button(action: back) {
                            Image(systemName: "chevron.left")
                                .font(.system(size: 18, weight: .black))
                                .foregroundColor(.white)
                                .frame(width: 44, height: 40)
                                .background(Slant(skew: 8).fill(Theme.ink))
                        }
                        Text(title)
                            .font(Theme.display(34))
                            .fontWidth(.condensed)
                            .foregroundColor(Theme.ink)
                    }
                    content
                }
                .padding(.leading, max(20, geo.safeAreaInsets.leading))
                .padding(.trailing, 18)
                .padding(.vertical, 16)
                .frame(width: geo.size.width * 0.5)
                .background(
                    LinearGradient(colors: [Theme.paper, Theme.paper.opacity(0.94), Theme.paper.opacity(0)], startPoint: .leading, endPoint: .trailing)
                        .ignoresSafeArea()
                )
                Spacer(minLength: 0)
            }
        }
    }
}

struct StatBar: View {
    let label: String
    let value: Float // ~0.8 ... 1.5
    var body: some View {
        HStack(spacing: 8) {
            Text(label).font(Theme.label(11, .heavy)).foregroundColor(Theme.muted).frame(width: 64, alignment: .leading)
            GeometryReader { g in
                ZStack(alignment: .leading) {
                    Capsule().fill(Theme.ink.opacity(0.08))
                    Capsule().fill(Theme.ink).frame(width: g.size.width * CGFloat(min(1, max(0.08, (value - 0.6) / 0.9))))
                }
            }
            .frame(height: 7)
        }
    }
}

extension View {
    func card(selected: Bool = false) -> some View {
        self
            .padding(12)
            .background(
                RoundedRectangle(cornerRadius: 14, style: .continuous)
                    .fill(Color.white)
                    .shadow(color: .black.opacity(selected ? 0.18 : 0.07), radius: selected ? 10 : 5, y: 3)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 14, style: .continuous)
                    .stroke(selected ? Theme.red : Color.clear, lineWidth: 3)
            )
    }
}
