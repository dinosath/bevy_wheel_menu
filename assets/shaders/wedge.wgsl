// Wedge / pie-segment shader for the bevy_wheel_menu pie shape.
//
// The UI node that uses this material is a square of size `outer_r * 2`.
// UV (0,0) is top-left, (1,1) is bottom-right; (0.5, 0.5) is the wheel centre.
//
// The shader converts UV into centred math coordinates (y-up) and checks whether
// the point falls inside the annular sector defined by [inner_r, outer_r] and
// [angle_start, angle_end] (both in radians, CCW from +X axis).

#import bevy_ui::ui_vertex_output::UiVertexOutput

struct WedgeParams {
    color: vec4<f32>,
    inner_r: f32,
    outer_r: f32,
    angle_start: f32,
    angle_end: f32,
}

@group(1) @binding(0)
var<uniform> params: WedgeParams;

const TAU: f32 = 6.28318530718;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // Convert UV to pixel offsets from wheel centre (+x right, +y up).
    let dia = params.outer_r * 2.0;
    let cx =  (in.uv.x - 0.5) * dia;
    let cy = -(in.uv.y - 0.5) * dia;   // flip y: UI y-down → math y-up

    let r = sqrt(cx * cx + cy * cy);

    // Radial bounds check.
    if r < params.inner_r || r > params.outer_r {
        return vec4<f32>(0.0);
    }

    // Compute angle in [0, TAU).
    var angle = atan2(cy, cx);
    if angle < 0.0 { angle = angle + TAU; }

    // Normalise a0 and a1 to [0, TAU).
    var a0 = params.angle_start;
    var a1 = params.angle_end;
    a0 = a0 - floor(a0 / TAU) * TAU;
    a1 = a1 - floor(a1 / TAU) * TAU;

    var inside: bool;
    if a0 <= a1 {
        inside = angle >= a0 && angle <= a1;
    } else {
        // Sector wraps across the 0/TAU boundary.
        inside = angle >= a0 || angle <= a1;
    }

    if !inside {
        return vec4<f32>(0.0);
    }

    return params.color;
}
