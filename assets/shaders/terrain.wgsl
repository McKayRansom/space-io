#import bevy_sprite::mesh2d_vertex_output::VertexOutput

// Must match TerrainMaterialUniform in Rust (std140, 112 bytes = 7×16):
//   colors: array<vec4<f32>, 6> = 96 bytes  (offset 0)
//   base_radius_frac: f32       =  4 bytes  (offset 96)
//   height_scale_frac: f32      =  4 bytes  (offset 100)
//   _pad0/_pad1: u32            =  8 bytes  (offset 104)
// Total: 112 bytes ✓
struct TerrainMaterialUniform {
    colors: array<vec4<f32>, 6>,
    base_radius_frac: f32,
    height_scale_frac: f32,
    pixels: f32,
    _pad0: u32,
    _pad1: u32,
}

@group(2) @binding(0) var<uniform> material: TerrainMaterialUniform;
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

    // Map UV to centered coordinates: (0,0) = center, ±1 = mesh edge
    var centered = in.uv - vec2<f32>(0.5);
    var r_normalized = length(centered) * 2.0;
    var angle = atan2(-centered.y, centered.x);

    var surface_r = terrain_surface_r(angle);

    // Discard pixels above the terrain surface (transparent sky)
    if r_normalized > surface_r {
        discard;
    }

    // Relcalc with 
    // Pixelate UV (expirement)
    let uv = floor(in.uv * material.pixels) / material.pixels;

    centered = uv - vec2<f32>(0.5);
    r_normalized = length(centered) * 2.0;
    angle = atan2(-centered.y, centered.x);

    surface_r = terrain_surface_r(angle);


    // Depth below surface, normalized to [0, 1] over the height_scale band
    let depth = (surface_r - r_normalized) / material.height_scale_frac;

    // Color selection based on depth:
    //   [0] bright surface, [1] shallow, [2] mid, [3] deep shadow,
    //   [4] deeper, [5] deepest core
    var col: vec4<f32>;
    col = material.colors[0];
    if depth < 0.05 {
        col = material.colors[0];
    } else if depth < 0.15 {
        col = mix(material.colors[0], material.colors[1], (depth - 0.05) / 0.10);
    } else if depth < 0.30 {
        col = mix(material.colors[1], material.colors[2], (depth - 0.15) / 0.15);
    } else if depth < 0.50 {
        col = mix(material.colors[2], material.colors[3], (depth - 0.30) / 0.20);
    } else if depth < 0.75 {
        col = mix(material.colors[3], material.colors[4], (depth - 0.50) / 0.25);
    } else {
        col = mix(material.colors[4], material.colors[5], clamp((depth - 0.75) / 0.25, 0.0, 1.0));
    }

    return vec4<f32>(col.rgb, col.a);
}
