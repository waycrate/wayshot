#version 300 es
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec2 aTexCoord;

out vec2 vTexCoord;

void main()
{
	gl_Position = vec4(aPos, 1.0);
	vTexCoord = vec2(aTexCoord.x, aTexCoord.y);
}