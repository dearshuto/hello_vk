#version 450

// layout(location = 0) in vec2 i_Position;

void main()
{
    if (gl_VertexIndex == 0)
    {
        gl_Position = vec4(0.0, 1.0, 0.0, 1.0);
    }
    else if (gl_VertexIndex == 1)
    {
        gl_Position = vec4(-1.0, -1.0, 0.0, 1.0);
    }
    else if (gl_VertexIndex == 2)
    {
        gl_Position = vec4(11.0, -1.0, 0.0, 1.0);
    } else
    {
        gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
    }
}
