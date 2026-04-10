#import bevy_sprite::mesh2d_vertex_output::VertexOutput

// Must match PlanetTerrainMaterialUniform in Rust (ShaderType layout, std140 rules):
// colors: array<vec4<f32>, 6> = 96 bytes  (offset 0)
// light_origin: vec2<f32>     =  8 bytes  (offset 96)
// pixels..should_dither       = 13×4=52b  (offset 104)
// _pad0, _pad1: u32           =  8 bytes  (offset 156)
// Total: 160 bytes = 10×16 ✓
struct PlanetTerrainMaterialUniform {
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
    base_radius_frac: f32,
    height_scale_frac: f32,
    atmosphere_radius: f32,
    _pad0: u32,
    _pad1: u32,
}

@group(2) @binding(0) var<uniform> material: PlanetTerrainMaterialUniform;

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

// Pre-computed heightmap: 1024×1 R32Float texture.
// U coordinate = (angle / TAU + 0.5), wraps via Repeat address mode.
@group(2) @binding(1) var heightmap_tex: texture_2d<f32>;
@group(2) @binding(2) var heightmap_sampler: sampler;

const TAU: f32 = 6.28318530718;

/// Returns the normalized terrain surface radius at a given angle.
fn terrain_surface_r(angle: f32) -> f32 {
    let tex_u = angle / TAU + 0.5;
    let width = i32(textureDimensions(heightmap_tex).x);
    let pixel_f = tex_u * f32(width);
    let p0 = i32(floor(pixel_f)) % width;
    let p1 = (p0 + 1) % width;
    let t = fract(pixel_f);
    let h0 = textureLoad(heightmap_tex, vec2<i32>(p0, 0), 0).r;
    let h1 = textureLoad(heightmap_tex, vec2<i32>(p1, 0), 0).r;
    let h = mix(h0, h1, t);
    return material.base_radius_frac + h * material.height_scale_frac;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {

    //*************** Terrain cutoff *************************
    
    // Map UV to centered coordinates: (0,0) = center, ±1 = mesh edge
    let centered = in.uv - vec2<f32>(0.5);
    let r_normalized = length(centered) * 2.0;
    let angle = atan2(-centered.y, centered.x);

    let surface_r = terrain_surface_r(angle);

    // Discard pixels above the terrain surface (transparent sky)
    if r_normalized > surface_r {
        if r_normalized > material.atmosphere_radius {
            discard;
        } else {
            var col = material.colors[4];
            let a = (material.atmosphere_radius - r_normalized) / (material.atmosphere_radius - material.base_radius_frac);
            return vec4<f32>(col.rgb, col.a * a);
        }
    }


    //*************** Planet Texture *************************

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
