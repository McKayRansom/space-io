#import bevy_sprite::mesh2d_vertex_output::VertexOutput

// Must match PlanetMaterialUniform in Rust (ShaderType layout, std140 rules):
// colors: array<vec4<f32>, 6> = 96 bytes  (offset 0)
// light_origin: vec2<f32>     =  8 bytes  (offset 96)
// pixels..should_dither       = 13×4=52b  (offset 104)
// _pad0, _pad1: u32           =  8 bytes  (offset 156)
// Total: 160 bytes = 10×16 ✓
struct PlanetMaterialUniform {
    colors: array<vec4<f32>, 6>,
    light_origin: vec2<f32>,
    pixels: f32,
    rotation: f32,
    time_speed: f32,
    dither_size: f32,
    light_border_1: f32,
    light_border_2: f32,
    river_cutoff: f32,
    size: f32,
    seed: f32,
    octaves: i32,
    time: f32,
    should_dither: u32,
    _pad0: u32,
    _pad1: u32,
}

@group(2) @binding(0) var<uniform> material: PlanetMaterialUniform;

// Tiled random — keeps continents stable as the planet rotates
fn rand(coord: vec2<f32>) -> f32 {
    let m = vec2<f32>(2.0, 1.0) * round(material.size);
    let tiled = coord - m * floor(coord / m);
    return fract(sin(dot(tiled, vec2<f32>(12.9898, 78.233))) * 15.5453 * material.seed);
}

fn noise(coord: vec2<f32>) -> f32 {
    let i = floor(coord);
    let f = fract(coord);
    let a = rand(i);
    let b = rand(i + vec2<f32>(1.0, 0.0));
    let c = rand(i + vec2<f32>(0.0, 1.0));
    let d = rand(i + vec2<f32>(1.0, 1.0));
    let cubic = f * f * (3.0 - 2.0 * f);
    return mix(a, b, cubic.x) + (c - a) * cubic.y * (1.0 - cubic.x) + (d - b) * cubic.x * cubic.y;
}

fn fbm(coord_in: vec2<f32>) -> f32 {
    var value = 0.0;
    var scale = 0.5;
    var coord = coord_in;
    for (var i = 0; i < 20; i++) {
        if i >= material.octaves { break; }
        value += noise(coord) * scale;
        coord *= 2.0;
        scale *= 0.5;
    }
    return value;
}

fn spherify(uv: vec2<f32>) -> vec2<f32> {
    let centered = uv * 2.0 - 1.0;
    let z = sqrt(max(0.0, 1.0 - dot(centered, centered)));
    let sphere = centered / (z + 1.0);
    return sphere * 0.5 + 0.5;
}

fn planet_rotate(coord_in: vec2<f32>, angle: f32) -> vec2<f32> {
    var coord = coord_in - vec2<f32>(0.5);
    let c = cos(angle);
    let s = sin(angle);
    coord = mat2x2<f32>(vec2<f32>(c, s), vec2<f32>(-s, c)) * coord;
    return coord + vec2<f32>(0.5);
}

// GLSL mod semantics (always non-negative for positive y)
fn glsl_mod(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}

fn dither(uv1: vec2<f32>, uv2: vec2<f32>) -> bool {
    let step = 2.0 / material.pixels;
    return glsl_mod(uv1.x + uv2.y, step) <= 1.0 / material.pixels;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Pixelize UV
    let uv_raw = in.uv;
    var uv = floor(uv_raw * material.pixels) / material.pixels;

    let dith = dither(uv, uv_raw);
    // Slightly less than 0.5 avoids edge pixel artifacts
    let a = step(length(uv - vec2<f32>(0.5)), 0.49999);

    // Map flat UV to sphere surface
    uv = spherify(uv);
    let d_light = distance(uv, material.light_origin);

    // Tilt/rotate the planet
    uv = planet_rotate(uv, material.rotation);

    // Scrolling FBM noise — time drives the rotation animation
    let base_fbm_uv = uv * material.size + vec2<f32>(material.time * material.time_speed, 0.0);

    var fbm1 = fbm(base_fbm_uv);
    var fbm2 = fbm(base_fbm_uv - material.light_origin * fbm1);
    var fbm3 = fbm(base_fbm_uv - material.light_origin * 1.5 * fbm1);
    var fbm4 = fbm(base_fbm_uv - material.light_origin * 2.0 * fbm1);

    let river_fbm_raw = fbm(base_fbm_uv + fbm1 * 6.0);
    let river_fbm = step(material.river_cutoff, river_fbm_raw);

    let dither_border = (1.0 / material.pixels) * material.dither_size;

    // Apply light-zone modifiers to fbm values (mirrors original Godot shader exactly)
    if d_light < material.light_border_1 {
        fbm4 *= 0.9;
    }
    if d_light > material.light_border_1 {
        fbm2 *= 1.05;
        fbm3 *= 1.05;
        fbm4 *= 1.05;
    }
    if d_light > material.light_border_2 {
        fbm2 *= 1.3;
        fbm3 *= 1.4;
        fbm4 *= 1.8;
        if d_light < material.light_border_2 + dither_border {
            // should_dither==0 means disabled — show solid edge instead of dithered
            if dith || material.should_dither == 0u {
                fbm4 *= 0.5;
            }
        }
    }

    // Increase contrast on lighting
    let d_light_c = pow(d_light, 2.0) * 0.4;

    // Color selection: land layers (0=bright, 3=shadow), then rivers (4=shallow, 5=deep)
    var col = material.colors[3];
    if fbm4 + d_light_c < fbm1 * 1.5 { col = material.colors[2]; }
    if fbm3 + d_light_c < fbm1 * 1.0  { col = material.colors[1]; }
    if fbm2 + d_light_c < fbm1        { col = material.colors[0]; }
    if river_fbm < fbm1 * 0.5 {
        col = material.colors[5];
        if fbm4 + d_light_c < fbm1 * 1.5 { col = material.colors[4]; }
    }

    return vec4<f32>(col.rgb, a * col.a);
}
