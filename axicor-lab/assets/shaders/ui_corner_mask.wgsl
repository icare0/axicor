struct CornerMaskMaterial {
    @location(0) color: vec4<f32>,
    @location(1) pivot: vec2<f32>, // (1,1) for TL, (0,1) for TR, (1,0) for BL, (0,0) for BR
};

@group(1) @binding(0)
var<uniform> material: CornerMaskMaterial;

@fragment
fn fragment(
    @location(0) uv: vec2<f32>,
) -> @location(0) vec4<f32> {
    let dist = distance(uv, material.pivot);
    
    // If distance to pivot is greater than 1.0 (normalized radius), output color.
    // Otherwise, output transparent.
    if (dist > 1.0) {
        return material.color;
    } else {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
}
