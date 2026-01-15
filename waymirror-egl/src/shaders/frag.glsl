#version 300 es

precision mediump float;
out vec4 FragColor;
uniform sampler2D uTexture;
in vec2 vTexCoord;

void main() {
   vec4 color = texture(uTexture, vTexCoord);
   FragColor = vec4 ( 1.0-color.r,1.0-color.g,1.0-color.b, 1.0 );
}
