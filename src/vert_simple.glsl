#version 330 core

uniform float width_adjustment;

layout (location = 0) in vec3 position;
layout (location = 1) in vec2 texcoord;

out vec2 f_texcoord;

void main() {
  gl_Position = vec4(position.xy, 0.0, 1.0);
  float scale_amount = clamp(width_adjustment, 0.0, 1.0);
  float nudge_amount = (1.0 - scale_amount) / 2.0;
  f_texcoord = vec2(texcoord.x * scale_amount + nudge_amount, texcoord.y);
}
