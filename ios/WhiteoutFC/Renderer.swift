import MetalKit
import SwiftUI
import FightCore

/// Uploads the Rust-generated low-poly mesh every frame and draws it in one call.
final class Renderer: NSObject, MTKViewDelegate {
    private let device: MTLDevice
    private let queue: MTLCommandQueue
    private let pipeline: MTLRenderPipelineState
    private let depth: MTLDepthStencilState
    private var buffers: [MTLBuffer] = []
    private var index = 0
    private let inflight = DispatchSemaphore(value: 3)
    private static let capacity = 160_000
    private var maxVertices: Int { Renderer.capacity }
    private var last = CACurrentMediaTime()
    private weak var core: GameCore?

    init?(view: MTKView, core: GameCore) {
        guard let device = MTLCreateSystemDefaultDevice(), let queue = device.makeCommandQueue() else { return nil }
        self.device = device
        self.queue = queue
        self.core = core
        view.device = device
        view.colorPixelFormat = .bgra8Unorm
        view.depthStencilPixelFormat = .depth32Float
        view.sampleCount = 4
        view.clearColor = MTLClearColor(red: 0.972, green: 0.973, blue: 0.978, alpha: 1)
        view.preferredFramesPerSecond = 60
        view.isPaused = false
        view.enableSetNeedsDisplay = false

        guard let lib = device.makeDefaultLibrary() else { return nil }
        let vd = MTLVertexDescriptor()
        vd.attributes[0].format = .float3; vd.attributes[0].offset = 0; vd.attributes[0].bufferIndex = 0
        vd.attributes[1].format = .float3; vd.attributes[1].offset = 12; vd.attributes[1].bufferIndex = 0
        vd.attributes[2].format = .float4; vd.attributes[2].offset = 24; vd.attributes[2].bufferIndex = 0
        vd.layouts[0].stride = MemoryLayout<FcVertex>.stride

        let pd = MTLRenderPipelineDescriptor()
        pd.vertexFunction = lib.makeFunction(name: "vmain")
        pd.fragmentFunction = lib.makeFunction(name: "fmain")
        pd.vertexDescriptor = vd
        pd.colorAttachments[0].pixelFormat = view.colorPixelFormat
        pd.depthAttachmentPixelFormat = view.depthStencilPixelFormat
        pd.rasterSampleCount = view.sampleCount
        guard let ps = try? device.makeRenderPipelineState(descriptor: pd) else { return nil }
        pipeline = ps

        let dd = MTLDepthStencilDescriptor()
        dd.depthCompareFunction = .less
        dd.isDepthWriteEnabled = true
        depth = device.makeDepthStencilState(descriptor: dd)!

        let bytes = Renderer.capacity * MemoryLayout<FcVertex>.stride
        var bufs: [MTLBuffer] = []
        for _ in 0..<3 {
            guard let b = device.makeBuffer(length: bytes, options: .storageModeShared) else { return nil }
            bufs.append(b)
        }
        self.buffers = bufs
        super.init()
    }

    func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {
        core?.resize(Float(size.width), Float(size.height))
    }

    func draw(in view: MTKView) {
        guard let core else { return }
        let now = CACurrentMediaTime()
        let dt = Float(min(now - last, 0.1))
        last = now
        let ds = view.drawableSize
        if ds.width > 0 { core.resize(Float(ds.width), Float(ds.height)) }
        core.tick(dt: dt)

        inflight.wait()
        var count: UInt32 = 0
        guard let src = fc_vertices(core.g, &count) else { inflight.signal(); return }
        let n = min(Int(count), maxVertices)
        let buf = buffers[index]
        index = (index + 1) % buffers.count
        memcpy(buf.contents(), src, n * MemoryLayout<FcVertex>.stride)
        var u = FcUniforms()
        fc_uniforms(core.g, &u)

        guard let pass = view.currentRenderPassDescriptor,
              let drawable = view.currentDrawable,
              let cmd = queue.makeCommandBuffer(),
              let enc = cmd.makeRenderCommandEncoder(descriptor: pass) else {
            inflight.signal()
            return
        }
        enc.setRenderPipelineState(pipeline)
        enc.setDepthStencilState(depth)
        enc.setCullMode(.none)
        enc.setVertexBuffer(buf, offset: 0, index: 0)
        enc.setVertexBytes(&u, length: MemoryLayout<FcUniforms>.stride, index: 1)
        enc.setFragmentBytes(&u, length: MemoryLayout<FcUniforms>.stride, index: 1)
        if n > 0 { enc.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: n) }
        enc.endEncoding()
        cmd.present(drawable)
        let sem = inflight
        cmd.addCompletedHandler { _ in sem.signal() }
        cmd.commit()
    }
}

/// SwiftUI host for the Metal view. In menus it also handles orbit / zoom / poke gestures.
struct MetalStage: UIViewRepresentable {
    @ObservedObject var core: GameCore

    func makeCoordinator() -> Coordinator { Coordinator(core: core) }

    func makeUIView(context: Context) -> MTKView {
        let v = MTKView(frame: .zero)
        context.coordinator.renderer = Renderer(view: v, core: core)
        v.delegate = context.coordinator.renderer
        let pan = UIPanGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.pan(_:)))
        let pinch = UIPinchGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.pinch(_:)))
        let tap = UITapGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.tap(_:)))
        [pan, pinch, tap].forEach { v.addGestureRecognizer($0) }
        return v
    }

    func updateUIView(_ uiView: MTKView, context: Context) {}

    final class Coordinator: NSObject {
        let core: GameCore
        var renderer: Renderer?
        private var lastPan = CGPoint.zero
        init(core: GameCore) { self.core = core }

        @objc func pan(_ g: UIPanGestureRecognizer) {
            let t = g.translation(in: g.view)
            if g.state == .began { lastPan = .zero }
            let dx = Float(t.x - lastPan.x), dy = Float(t.y - lastPan.y)
            lastPan = t
            core.orbit(-dx * 0.008, dy * 0.004)
        }
        @objc func pinch(_ g: UIPinchGestureRecognizer) {
            core.zoom(Float(1 / g.scale))
            g.scale = 1
        }
        @objc func tap(_ g: UITapGestureRecognizer) {
            guard let v = g.view else { return }
            let p = g.location(in: v)
            let x = Float(p.x / v.bounds.width * 2 - 1)
            let y = Float(1 - p.y / v.bounds.height * 2)
            core.poke(x, y)
        }
    }
}
