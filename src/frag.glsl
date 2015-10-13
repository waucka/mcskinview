#version 330 core

uniform uint time;
uniform sampler2D tex;

in vec2 f_texcoord;

out vec4 color;

void main() {
  color = texture(tex, vec2(f_texcoord.s, f_texcoord.t));
}
