#version 300 es

precision mediump float;
out vec4 FragColor;
uniform sampler2D uTexture;
in vec2 vTexCoord;

void main() {
   vec4 color = texture(uTexture, vTexCoord);
   FragColor = vec4 (color.r, color.g, color.b, 1.0 );
}
