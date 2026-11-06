#include <metal_stdlib>
using namespace metal;

// Must match FcVertex / FcUniforms in fightcore.h
struct VIn {
    float3 pos [[attribute(0)]];
    float3 nrm [[attribute(1)]];
    float4 col [[attribute(2)]];
};

struct Uniforms {
    float4x4 viewProj;
    float4 camPos;
    float4 lightDir;
    float4 params; // time, fogStart, fogEnd, flash
};

struct VOut {
    float4 position [[position]];
    float3 world;
    float3 nrm;
    float4 col;
};

vertex VOut vmain(VIn in [[stage_in]], constant Uniforms &u [[buffer(1)]]) {
    VOut o;
    o.position = u.viewProj * float4(in.pos, 1.0);
    o.world = in.pos;
    o.nrm = in.nrm;
    o.col = in.col;
    return o;
}

constant float3 FOG = float3(0.972, 0.973, 0.978);

fragment float4 fmain(VOut in [[stage_in]], constant Uniforms &u [[buffer(1)]]) {
    float3 n = normalize(in.nrm);
    float3 L = u.lightDir.xyz;
    float ndl = max(dot(n, L), 0.0);
    float t = n.y * 0.5 + 0.5;
    float3 amb = mix(float3(0.78, 0.74, 0.72), float3(0.92, 0.95, 1.0), t);
    float3 V = normalize(u.camPos.xyz - in.world);
    float rim = pow(1.0 - max(dot(n, V), 0.0), 3.0) * 0.18;
    float3 albedo = in.col.rgb;
    float3 litc = albedo * (amb * 0.62 + float3(1.0, 0.97, 0.92) * ndl * 0.55) + rim;
    float3 c = mix(albedo, litc, in.col.a);
    float dist = length(in.world - u.camPos.xyz);
    float fog = smoothstep(u.params.y, u.params.z, dist);
    c = mix(c, FOG, fog);
    c = mix(c, float3(1.0), u.params.w * 0.3);
    return float4(saturate(c), 1.0);
}
