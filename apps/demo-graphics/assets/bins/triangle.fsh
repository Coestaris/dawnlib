#version 330 core
#extension GL_ARB_explicit_uniform_location : enable

in vec2 TexCoord;

out vec4 FragColor;
uniform sampler2D texture1;

void main()
{
    FragColor = texture(texture1, TexCoord);
    FragColor += vec4(1.0, 0.0, 0.0, 1.0); // Red color for testing
}