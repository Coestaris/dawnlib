#version 330 core
#extension GL_ARB_explicit_uniform_location : enable

in vec3 Normal;
out vec4 FragColor;

void main()
{
    FragColor = vec4(Normal, 0) + vec4(0.5, 0.5, 0.5, 1.0);
}