#version 330 core

uniform mat4 projection;
uniform mat4 view;
uniform mat4 model;

layout (location = 0) in vec3 position;
layout (location = 1) in vec2 texcoord;
layout (location = 2) in vec3 normal;

out vec2 f_texcoord;

void main() {
  gl_Position = projection * view * model * vec4(position, 1.0);
  f_texcoord = texcoord;
}
