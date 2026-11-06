import SwiftUI

@main
struct WhiteoutFCApp: App {
    @StateObject private var core = GameCore()
    @Environment(\.scenePhase) private var phase

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(core)
                .statusBarHidden(true)
                .persistentSystemOverlays(.hidden)
                .preferredColorScheme(.light)
        }
        .onChange(of: phase) { p in
            if p != .active {
                core.save()
                if core.screen == .fight && core.result == nil { core.paused = true }
            }
        }
    }
}

struct RootView: View {
    @EnvironmentObject var core: GameCore

    var body: some View {
        ZStack {
            MetalStage(core: core)
                .ignoresSafeArea()
            Group {
                switch core.screen {
                case .menu: MainMenuView()
                case .ladder: LadderView()
                case .shop: ShopView()
                case .fighters: FightersView()
                case .celebrations: CelebrationsView()
                case .stats: StatsView()
                case .fight: FightView()
                }
            }
            .transition(.opacity.combined(with: .move(edge: .leading)))
        }
        .animation(.easeOut(duration: 0.25), value: core.screen)
        .background(Theme.paper)
    }
}
