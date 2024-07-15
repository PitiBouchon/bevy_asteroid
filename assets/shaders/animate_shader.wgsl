#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_types
#import bevy_sprite::mesh2d_view_bindings::globals

@group(2) @binding(0) var<uniform> material_color: vec4<f32>;

fn hash23(p: vec2<f32>) -> vec3<f32> {
  let q = vec3<f32>(dot(p, vec2<f32>(127.1, 311.7)),
      dot(p, vec2<f32>(269.5, 183.3)),
      dot(p, vec2<f32>(419.2, 371.9)));
  return fract(sin(q) * 43758.5453);
}

fn voro_noise_2d(x: vec2<f32>, u: f32, v: f32) -> f32 {
  let p = floor(x);
  let f = fract(x);
  let k = 1. + 63. * pow(1. - v, 4.);
  var va: f32 = 0.;
  var wt: f32 = 0.;
  for(var j: i32 = -2; j <= 2; j = j + 1) {
    for(var i: i32 = -2; i <= 2; i = i + 1) {
      let g = vec2<f32>(f32(i), f32(j));
      let o = hash23(p + g) * vec3<f32>(u, u, 1.);
      let r = g - f + o.xy;
      let d = dot(r, r);
      let ww = pow(1. - smoothstep(0., 1.414, sqrt(d)), k);
      va = va + o.z * ww;
      wt = wt + ww;
    }
  }
  return va / wt;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let laser_speed = vec2(20.0, 800.0);
    let laser_scale = 20.0;
    let laser_thickness = 30.0;
    let laser_edge_smoothness = 10.0;
    let time = globals.time;
    let variation = 2.0;

    let c1 = pow(mesh.uv.r, laser_edge_smoothness + variation * sin(mesh.uv.y * 30.0 + time * 40.0));
    let c2 = pow(1.0 - mesh.uv.r, laser_edge_smoothness + variation * sin(mesh.uv.y * 30.0 + time * 40.0 + cos(time)));

    let c3 = 1.0 - clamp(c1 + c2, 0.0, 1.0);
    let c4 = pow(c3, laser_thickness);

    let v = voro_noise_2d(
    vec2(
        mesh.uv.x * laser_scale / 100.0, // introduce some noise
        mesh.uv.y * laser_scale + time * laser_speed.y
    ), 2.0, 1.0) * 0.6 + 0.2;

    let c5 = c4 * v;
    
    let c6 = c5 * vec4(material_color.x, material_color.y, material_color.z, material_color.a);

    return c6;
}
